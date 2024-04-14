#![allow(clippy::missing_safety_doc)]
#![allow(clippy::type_complexity)]

use std::{
    any::{Any, TypeId},
    cell::Cell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread::LocalKey,
};

use autoken::{cap, BorrowsMut, BorrowsRef, CapTarget, TokenSet};
use generational_arena::{Arena, Index};
use rustc_hash::FxHashMap;

use super::{
    deferred::{DeferQueue, DeferQueueCap},
    lock::UnpoisonExt,
};

// === Universe === //

cap! {
    pub UniverseCapability = Universe;
}

// We could theoretically use a `Box` here since we never actually duplicate these `Arc`s but that
// would have bad noalias semantics.
pub type UniverseComponentMap = FxHashMap<TypeId, Arc<RwLock<dyn Any + Send + Sync>>>;

#[derive(Default)]
pub struct Universe {
    components: Mutex<UniverseComponentMap>,

    // TODO: Optimize and clean up
    entities: Mutex<Arena<FxHashMap<TypeId, Index>>>,
}

impl fmt::Debug for Universe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Universe").finish_non_exhaustive()
    }
}

impl Universe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_with_queue<L: ComponentList, R>(
        &self,
        queue: &mut DeferQueue,
        f: impl FnOnce() -> R,
    ) -> R {
        self.run::<L, R>(|| DeferQueueCap::provide(queue, f))
    }

    pub fn run<L: ComponentList, R>(&self, f: impl FnOnce() -> R) -> R {
        unsafe {
            // Rust-Analyzer doesn't like this expression for some reason.
            #[allow(clippy::explicit_auto_deref)]
            let _guard = L::apply(&mut *self.components.lock().unpoison());

            autoken::absorb::<L::TokensMut, R>(|| {
                fn dummy_borrow_set<'a, T: TokenSet>() -> &'a () {
                    autoken::tie!('a => set T);
                    &()
                }

                let all_borrow = dummy_borrow_set::<L::TokensMut>();
                let res = autoken::absorb::<L::Tokens, R>(|| UniverseCapability::provide(self, f));
                let _ = all_borrow;
                res
            })
        }
    }
}

pub struct UniverseAcquires<L: ComponentList>(PhantomData<fn() -> L>);

impl<'a, L: ComponentList> CapTarget<&'a Universe> for UniverseAcquires<L> {
    fn provide<R>(universe: &'a Universe, f: impl FnOnce() -> R) -> R {
        universe.run::<L, R>(f)
    }
}

fn get_component_in_map<T: Component>(
    universe: &mut UniverseComponentMap,
) -> &RwLock<dyn Any + Send + Sync> {
    universe
        .entry(TypeId::of::<T>())
        .or_insert_with(|| Arc::new(RwLock::new(Arena::<T>::new())))
}

// === CompTokensOf === //

pub type CompBorrowsRef<'a, T> = BorrowsRef<'a, CompTokensOf<T>>;

pub type CompBorrowsMut<'a, T> = BorrowsMut<'a, CompTokensOf<T>>;

pub type CompTokensOf<T> = (
    <T as ComponentList>::Tokens,
    autoken::Ref<UniverseCapability>,
);

// === ComponentList === //

pub struct ComponentArenaToken<T> {
    _ty: PhantomData<fn() -> T>,
}

pub unsafe trait ComponentList {
    type Tokens: TokenSet;
    type TokensMut: TokenSet;
    type ApplyGuard;

    fn iter() -> impl Iterator<Item = (TypeId, bool)>;

    unsafe fn apply(universe: &mut UniverseComponentMap) -> Self::ApplyGuard;
}

unsafe impl<T: Component> ComponentList for &'_ T {
    type Tokens = autoken::Ref<ComponentArenaToken<T>>;
    type TokensMut = autoken::Mut<ComponentArenaToken<T>>;
    type ApplyGuard = (
        RwLockReadGuard<'static, dyn Any + Send + Sync>,
        ComponentScopeGuard<T>,
    );

    fn iter() -> impl Iterator<Item = (TypeId, bool)> {
        [(TypeId::of::<T>(), false)].into_iter()
    }

    unsafe fn apply(universe: &mut UniverseComponentMap) -> Self::ApplyGuard {
        let guard = get_component_in_map::<T>(universe).read().unpoison();
        let ptr = (&*guard) as *const (dyn Any + Send + Sync) as *const Arena<T> as *mut Arena<T>;

        (
            std::mem::transmute::<
                RwLockReadGuard<'_, dyn Any + Send + Sync>,
                RwLockReadGuard<'static, dyn Any + Send + Sync>,
            >(guard),
            ComponentScopeGuard::new(ptr),
        )
    }
}

unsafe impl<T: Component> ComponentList for &'_ mut T {
    type Tokens = autoken::Mut<ComponentArenaToken<T>>;
    type TokensMut = autoken::Mut<ComponentArenaToken<T>>;
    type ApplyGuard = (
        RwLockWriteGuard<'static, dyn Any + Send + Sync>,
        ComponentScopeGuard<T>,
    );

    fn iter() -> impl Iterator<Item = (TypeId, bool)> {
        [(TypeId::of::<T>(), true)].into_iter()
    }

    unsafe fn apply(universe: &mut UniverseComponentMap) -> Self::ApplyGuard {
        let mut guard = get_component_in_map::<T>(universe).write().unpoison();
        let ptr = (&mut *guard) as *mut (dyn Any + Send + Sync) as *mut Arena<T>;

        (
            std::mem::transmute::<
                RwLockWriteGuard<'_, dyn Any + Send + Sync>,
                RwLockWriteGuard<'static, dyn Any + Send + Sync>,
            >(guard),
            ComponentScopeGuard::new(ptr),
        )
    }
}

unsafe impl ComponentList for () {
    type Tokens = ();
    type TokensMut = ();
    type ApplyGuard = ();

    fn iter() -> impl Iterator<Item = (TypeId, bool)> {
        [].into_iter()
    }

    unsafe fn apply(_universe: &mut UniverseComponentMap) -> Self::ApplyGuard {}
}

macro_rules! impl_component_list {
    () => {};
    ($first:ident $($rest:ident)*) => {
        unsafe impl<$first: ComponentList $(, $rest: ComponentList)*> ComponentList for ($first, $($rest,)*) {
            type Tokens = ($first::Tokens, $($rest::Tokens,)*);
            type TokensMut = ($first::TokensMut, $($rest::TokensMut,)*);
            type ApplyGuard = ($first::ApplyGuard, $($rest::ApplyGuard,)*);

            fn iter() -> impl Iterator<Item = (TypeId, bool)> {
                $first::iter() $(.chain($rest::iter()))*
            }

            unsafe fn apply(universe: &mut UniverseComponentMap) -> Self::ApplyGuard {
                ($first::apply(universe), $($rest::apply(universe),)*)
            }
        }

        impl_component_list!($($rest)*);
    };
}

impl_component_list!(T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12);

// === Component === //

pub unsafe trait Component: 'static + Sized + Send + Sync {
    unsafe fn tls() -> &'static LocalKey<Cell<*mut Arena<Self>>>;

    fn arena<'a>() -> &'a Arena<Self> {
        autoken::tie!('a => ref ComponentArenaToken<Self>);
        unsafe { &*Self::tls().get() }
    }

    fn arena_mut<'a>() -> &'a mut Arena<Self> {
        autoken::tie!('a => mut ComponentArenaToken<Self>);
        unsafe { &mut *Self::tls().get() }
    }
}

pub struct ComponentScopeGuard<T: Component> {
    _ty: PhantomData<fn() -> T>,
    old: *mut Arena<T>,
}

impl<T: Component> ComponentScopeGuard<T> {
    pub unsafe fn new(ptr: *mut Arena<T>) -> Self {
        Self {
            _ty: PhantomData,
            old: T::tls().replace(ptr),
        }
    }
}

impl<T: Component> Drop for ComponentScopeGuard<T> {
    fn drop(&mut self) {
        unsafe { T::tls().set(self.old) }
    }
}

#[doc(hidden)]
pub mod component_internals {
    pub use {
        super::Component,
        generational_arena::Arena,
        std::{cell::Cell, ptr::null_mut, thread::LocalKey, thread_local},
    };
}

#[macro_export]
macro_rules! component {
    ($($ty:ty),*$(,)?) => {$(
        unsafe impl $crate::util::arena::component_internals::Component for $ty {
            unsafe fn tls() -> &'static $crate::util::arena::component_internals::LocalKey<
                $crate::util::arena::component_internals::Cell<
                    *mut $crate::util::arena::component_internals::Arena<Self>,
                >>
            {
                $crate::util::arena::component_internals::thread_local! {
                    static TLS: $crate::util::arena::component_internals::Cell<
                        *mut $crate::util::arena::component_internals::Arena<$ty>,
                    > = const {
                        $crate::util::arena::component_internals::Cell::new(
                            $crate::util::arena::component_internals::null_mut(),
                        )
                    };
                }

                &TLS
            }
        }
    )*};
}

// === Entity === //

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Entity {
    index: Index,
}

impl Entity {
    pub fn insert<T: Component>(self, comp: T) -> Obj<T> {
        self.insert_obj(Obj::new(comp))
    }

    pub fn insert_obj<T: Component>(self, comp: Obj<T>) -> Obj<T> {
        let universe = UniverseCapability::get(|v| v).0;
        universe
            .entities
            .lock()
            .unpoison()
            .get_mut(self.index)
            .expect("entity is dead")
            .insert(TypeId::of::<T>(), comp.index);

        comp
    }

    pub fn try_get<T: Component>(self) -> Option<Obj<T>> {
        let universe = UniverseCapability::get(|v| v).0;
        universe
            .entities
            .lock()
            .unpoison()
            .get(self.index)
            .and_then(|map| map.get(&TypeId::of::<T>()))
            .map(|&index| Obj::from_index(index))
    }

    pub fn get<T: Component>(self) -> Obj<T> {
        self.try_get::<T>().expect("failed to fetch component")
    }

    pub fn is_alive(self) -> bool {
        let universe = UniverseCapability::get(|v| v).0;
        universe.entities.lock().unpoison().contains(self.index)
    }

    pub fn try_upgrade(self) -> Option<StrongEntity> {
        // TODO: Reference counting
        self.is_alive().then_some(StrongEntity { entity: self })
    }

    pub fn upgrade(self) -> StrongEntity {
        self.try_upgrade()
            .expect("attempted to upgrade dead entity")
    }
}

// === Obj === //

#[repr(transparent)]
pub struct Obj<T> {
    _ty: PhantomData<fn() -> T>,
    index: Index,
}

impl<T> fmt::Debug for Obj<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Obj").finish_non_exhaustive()
    }
}

impl<T> Copy for Obj<T> {}

impl<T> Clone for Obj<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Component> Obj<T> {
    pub fn new(value: T) -> Self {
        Self::from_index(T::arena_mut().insert(value))
    }

    pub fn destroy(me: Self) {
        T::arena_mut().remove(me.index);
    }

    #[allow(clippy::should_implement_trait)]
    pub fn deref<'a>(self) -> &'a T {
        autoken::tie!('a => ref ComponentArenaToken<T>);
        &T::arena()[self.index]
    }

    #[allow(clippy::should_implement_trait)]
    pub fn deref_mut<'a>(self) -> &'a mut T {
        autoken::tie!('a => mut ComponentArenaToken<T>);
        &mut T::arena_mut()[self.index]
    }
}

impl<T> Obj<T> {
    pub fn from_index(index: Index) -> Self {
        Self {
            _ty: PhantomData,
            index,
        }
    }

    pub fn index(me: Self) -> Index {
        me.index
    }
}

impl<T: Component> Deref for Obj<T> {
    type Target = T;

    fn deref<'a>(&'a self) -> &'a Self::Target {
        autoken::tie!('a => ref ComponentArenaToken<T>);
        (*self).deref()
    }
}

impl<T: Component> DerefMut for Obj<T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut Self::Target {
        autoken::tie!('a => mut ComponentArenaToken<T>);
        (*self).deref_mut()
    }
}

// === StrongEntity === //

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct StrongEntity {
    entity: Entity,
}

impl Default for StrongEntity {
    fn default() -> Self {
        Self::new()
    }
}

impl StrongEntity {
    pub fn new() -> Self {
        let universe = UniverseCapability::get(|v| v).0;
        let index = universe
            .entities
            .lock()
            .unpoison()
            .insert(FxHashMap::default());

        Self {
            entity: Entity { index },
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn split_guard(self) -> (StrongEntity, Entity) {
        let entity = self.entity;
        (self, entity)
    }

    pub fn insert<T: Component>(&self, comp: T) -> Obj<T> {
        self.entity.insert(comp)
    }

    pub fn insert_obj<T: Component>(&self, comp: Obj<T>) -> Obj<T> {
        self.entity.insert_obj(comp)
    }

    pub fn try_get<T: Component>(&self) -> Option<Obj<T>> {
        self.entity.try_get()
    }

    pub fn get<T: Component>(&self) -> Obj<T> {
        self.entity.get()
    }
}

impl Drop for StrongEntity {
    fn drop(&mut self) {
        // TODO: Reference counting
    }
}

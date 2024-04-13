#![allow(clippy::missing_safety_doc)]

use std::{
    any::{Any, TypeId},
    cell::Cell,
    fmt, hash,
    marker::{PhantomData, Unsize},
    ops::{Deref, DerefMut},
    ptr::{from_raw_parts, metadata, Pointee},
    sync::{Arc, LockResult, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread::LocalKey,
};

use autoken::{BorrowsMut, BorrowsRef, CapTarget, TokenSet};
use generational_arena::{Arena, Index};
use rustc_hash::FxHashMap;

// === World === //

// We could theoretically use a `Box` here since we never actually duplicate these `Arc`s but that
// would have bad noalias semantics.
pub type WorldComponentMap = FxHashMap<TypeId, Arc<RwLock<dyn Any + Send + Sync>>>;

#[derive(Default)]
pub struct World {
    components: Mutex<WorldComponentMap>,
}

impl fmt::Debug for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("World").finish_non_exhaustive()
    }
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn acquire<L: ComponentList, R>(&self, f: impl FnOnce() -> R) -> R {
        unsafe {
            // Rust-Analyzer doesn't like this expression for some reason.
            #[allow(clippy::explicit_auto_deref)]
            let _guard = L::apply(&mut *unpoison(self.components.lock()));

            autoken::absorb::<L::TokensMut, R>(|| {
                fn dummy_borrow_set<'a, T: TokenSet>() -> &'a () {
                    autoken::tie!('a => set T);
                    &()
                }

                let all_borrow = dummy_borrow_set::<L::TokensMut>();
                let res = autoken::absorb::<L::Tokens, R>(f);
                let _ = all_borrow;
                res
            })
        }
    }
}

pub struct WorldAcquires<L: ComponentList>(PhantomData<fn() -> L>);

impl<'a, L: ComponentList> CapTarget<&'a World> for WorldAcquires<L> {
    fn provide<R>(world: &'a World, f: impl FnOnce() -> R) -> R {
        world.acquire::<L, R>(f)
    }
}

fn get_component_in_map<T: Component>(
    world: &mut WorldComponentMap,
) -> &RwLock<dyn Any + Send + Sync> {
    world
        .entry(TypeId::of::<T>())
        .or_insert_with(|| Arc::new(RwLock::new(Arena::<T>::new())))
}

fn unpoison<G>(guard: LockResult<G>) -> G {
    match guard {
        Ok(guard) => guard,
        Err(err) => err.into_inner(),
    }
}

// === ComponentList === //

pub type CompBorrowsRef<'a, T> = BorrowsRef<'a, CompTokensOf<T>>;
pub type CompBorrowsMut<'a, T> = BorrowsMut<'a, CompTokensOf<T>>;
pub type CompTokensOf<T> = <T as ComponentList>::Tokens;

pub struct ComponentArenaToken<T> {
    _ty: PhantomData<fn() -> T>,
}

pub unsafe trait ComponentList {
    type Tokens: TokenSet;
    type TokensMut: TokenSet;
    type ApplyGuard;

    fn iter() -> impl Iterator<Item = (TypeId, bool)>;

    unsafe fn apply(world: &mut WorldComponentMap) -> Self::ApplyGuard;
}

unsafe impl<T: Component> ComponentList for &'static T {
    type Tokens = autoken::Ref<ComponentArenaToken<T>>;
    type TokensMut = autoken::Mut<ComponentArenaToken<T>>;
    type ApplyGuard = (
        RwLockReadGuard<'static, dyn Any + Send + Sync>,
        ComponentScopeGuard<T>,
    );

    fn iter() -> impl Iterator<Item = (TypeId, bool)> {
        [(TypeId::of::<T>(), false)].into_iter()
    }

    unsafe fn apply(world: &mut WorldComponentMap) -> Self::ApplyGuard {
        let guard = unpoison(get_component_in_map::<T>(world).read());
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

unsafe impl<T: Component> ComponentList for &'static mut T {
    type Tokens = autoken::Mut<ComponentArenaToken<T>>;
    type TokensMut = autoken::Mut<ComponentArenaToken<T>>;
    type ApplyGuard = (
        RwLockWriteGuard<'static, dyn Any + Send + Sync>,
        ComponentScopeGuard<T>,
    );

    fn iter() -> impl Iterator<Item = (TypeId, bool)> {
        [(TypeId::of::<T>(), true)].into_iter()
    }

    unsafe fn apply(world: &mut WorldComponentMap) -> Self::ApplyGuard {
        let mut guard = unpoison(get_component_in_map::<T>(world).write());
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

    unsafe fn apply(_world: &mut WorldComponentMap) -> Self::ApplyGuard {}
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

            unsafe fn apply(world: &mut WorldComponentMap) -> Self::ApplyGuard {
                ($first::apply(world), $($rest::apply(world),)*)
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

    pub fn destroy(self) {
        T::arena_mut().remove(self.index);
    }

    pub fn get<'a>(self) -> &'a T {
        autoken::tie!('a => ref ComponentArenaToken<T>);
        &T::arena()[self.index]
    }

    pub fn get_mut<'a>(self) -> &'a mut T {
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

    pub fn index(self) -> Index {
        self.index
    }
}

impl<T: Component> Deref for Obj<T> {
    type Target = T;

    fn deref<'a>(&'a self) -> &'a Self::Target {
        autoken::tie!('a => ref ComponentArenaToken<T>);
        self.get()
    }
}

impl<T: Component> DerefMut for Obj<T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut Self::Target {
        autoken::tie!('a => mut ComponentArenaToken<T>);

        self.get_mut()
    }
}

// === DynObj === //

pub struct DynObj<T: ?Sized> {
    index: Index,
    metadata: <T as Pointee>::Metadata,
}

impl<T: ?Sized> fmt::Debug for DynObj<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynObj").finish_non_exhaustive()
    }
}

impl<T: ?Sized> Copy for DynObj<T> {}

impl<T: ?Sized> Clone for DynObj<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> hash::Hash for DynObj<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T: ?Sized> Eq for DynObj<T> {}

impl<T: ?Sized> PartialEq for DynObj<T> {
    fn eq(&self, other: &DynObj<T>) -> bool {
        self.index == other.index
    }
}

impl<T: ?Sized> DynObj<T> {
    pub fn new<V>(value: Obj<V>) -> Self
    where
        Obj<V>: Unsize<T>,
    {
        let metadata = metadata(&value as &T);

        Self {
            index: value.index,
            metadata,
        }
    }
}

impl<T: ?Sized> Deref for DynObj<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*from_raw_parts(&self.index as *const Index as *const (), self.metadata) }
    }
}

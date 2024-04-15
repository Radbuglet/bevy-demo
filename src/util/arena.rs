#![allow(clippy::missing_safety_doc)]

use std::{
    cell::Cell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    thread::LocalKey,
};

use autoken::{BorrowsMut, BorrowsRef, TokenSet};
use bevy_ecs::{
    component::{ComponentId, Tick},
    system::{Resource, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use generational_arena::{Arena, Index};

// === RandomArena === //

#[derive(Debug, Resource)]
pub struct RandomArena<T> {
    arena: Arena<T>,
}

impl<T> Default for RandomArena<T> {
    fn default() -> Self {
        Self {
            arena: Default::default(),
        }
    }
}

// === RandomAccess === //

pub struct RandomAccess<'w, 's, L: RandomComponentList> {
    world: UnsafeWorldCell<'w>,
    state: &'s L::ParamState,
}

unsafe impl<'w2, 's2, L: RandomComponentList> SystemParam for RandomAccess<'w2, 's2, L> {
    type State = L::ParamState;
    type Item<'w, 's> = RandomAccess<'w, 's, L>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        // Fetch the IDs of each component used in the borrow and ensure that they don't conflict
        // with another parameter's borrow access.
        let state = L::get_param_state(world, system_meta);

        // Adjust the borrow set of this system.
        L::update_access_sets(&state, world, system_meta);

        state
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        RandomAccess {
            state: &*state,
            world,
        }
    }
}

impl<'w, 's, L: RandomComponentList> RandomAccess<'w, 's, L> {
    pub fn provide<R>(&mut self, f: impl FnOnce() -> R) -> R {
        unsafe {
            autoken::absorb::<L::TokensMut, R>(|| {
                let new_snap = L::tls_snapshot_from_world(self.state, self.world);
                let _guard = scopeguard::guard(L::fetch_tls_snapshot(), |snap| {
                    L::apply_tls_snapshot(&snap);
                });
                L::apply_tls_snapshot(&new_snap);

                fn dummy<'a, S: TokenSet>() -> &'a () {
                    autoken::tie!('a => set S);
                    &()
                }

                let _all = dummy::<L::TokensMut>();
                autoken::absorb::<L::Tokens, R>(f)
            })
        }
    }
}

// === RandomComponentList === //

pub type CompBorrowsRef<'a, T> = BorrowsRef<'a, CompTokensOf<T>>;

pub type CompBorrowsMut<'a, T> = BorrowsMut<'a, CompTokensOf<T>>;

pub type CompTokensOf<T> = <T as RandomComponentList>::Tokens;

pub unsafe trait RandomComponentList {
    /// The set of tokens absorbed by the list.
    type Tokens: TokenSet;

    /// The set of tokens absorbed by the list but each token is promoted to its mutable form.
    type TokensMut: TokenSet;

    /// The state of our [`RandomAccess`] system parameter.
    type ParamState: 'static + Copy + Send + Sync;

    type TlsSnapshot: 'static + Copy;

    /// Fetches the set of [`ComponentId`]s that this component list, ensuring that the existing
    /// system meta doesn't have any conflicting borrows.
    fn get_param_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::ParamState;

    /// Appends this set's resource set to the system metadata.
    fn update_access_sets(
        state: &Self::ParamState,
        world: &mut World,
        system_meta: &mut SystemMeta,
    );

    /// Fetch a snapshot of all previous arena TLS states.
    fn fetch_tls_snapshot() -> Self::TlsSnapshot;

    /// Compute new snapshot from world resources.
    unsafe fn tls_snapshot_from_world(
        state: &Self::ParamState,
        world: UnsafeWorldCell<'_>,
    ) -> Self::TlsSnapshot;

    /// Applies a snapshot on arena TLS states.
    unsafe fn apply_tls_snapshot(snap: &Self::TlsSnapshot);
}

unsafe impl<T: RandomComponent> RandomComponentList for &'_ T {
    type Tokens = autoken::Ref<RandomComponentToken<T>>;
    type TokensMut = autoken::Mut<RandomComponentToken<T>>;
    type ParamState = ComponentId;
    type TlsSnapshot = *mut RandomArena<T>;

    fn get_param_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::ParamState {
        let component_id = world.init_resource::<RandomArena<T>>();

        // TODO
        // let combined_access = system_meta.component_access_set.combined_access();
        // assert!(
        //     !combined_access.has_write(component_id),
        //     "error[B0002]: Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access.",
        //     std::any::type_name::<T>(),
        //     system_meta.name(),
        // );

        component_id
    }

    fn update_access_sets(
        &component_id: &Self::ParamState,
        world: &mut World,
        system_meta: &mut SystemMeta,
    ) {
        // TODO
        //         system_meta
        //             .component_access_set
        //             .add_unfiltered_read(component_id);
        //
        //         let archetype_component_id = world
        //             .get_resource_archetype_component_id(component_id)
        //             .unwrap();
        //
        //         system_meta
        //             .archetype_component_access
        //             .add_read(archetype_component_id);
    }

    fn fetch_tls_snapshot() -> Self::TlsSnapshot {
        unsafe { T::tls().get() }
    }

    unsafe fn tls_snapshot_from_world(
        &state: &Self::ParamState,
        world: UnsafeWorldCell<'_>,
    ) -> Self::TlsSnapshot {
        world
            .get_resource_by_id(state)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested does not exist: {}",
                    std::any::type_name::<T>()
                )
            })
            .as_ptr()
            .cast()
    }

    unsafe fn apply_tls_snapshot(&snap: &Self::TlsSnapshot) {
        unsafe { T::tls().set(snap) }
    }
}

unsafe impl<T: RandomComponent> RandomComponentList for &'_ mut T {
    type Tokens = autoken::Mut<RandomComponentToken<T>>;
    type TokensMut = autoken::Mut<RandomComponentToken<T>>;
    type ParamState = ComponentId;
    type TlsSnapshot = *mut RandomArena<T>;

    fn get_param_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::ParamState {
        let component_id = world.init_resource::<RandomArena<T>>();

        // TODO
        // let combined_access = system_meta.component_access_set.combined_access();
        // assert!(
        //     !combined_access.has_write(component_id),
        //     "error[B0002]: Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access.",
        //     std::any::type_name::<T>(),
        //     system_meta.name(),
        // );

        component_id
    }

    fn update_access_sets(
        state: &Self::ParamState,
        world: &mut World,
        system_meta: &mut SystemMeta,
    ) {
        // TODO
        //         system_meta
        //             .component_access_set
        //             .add_unfiltered_read(component_id);
        //
        //         let archetype_component_id = world
        //             .get_resource_archetype_component_id(component_id)
        //             .unwrap();
        //
        //         system_meta
        //             .archetype_component_access
        //             .add_read(archetype_component_id);
    }

    fn fetch_tls_snapshot() -> Self::TlsSnapshot {
        unsafe { T::tls().get() }
    }

    unsafe fn tls_snapshot_from_world(
        &state: &Self::ParamState,
        world: UnsafeWorldCell<'_>,
    ) -> Self::TlsSnapshot {
        world
            .get_resource_by_id(state)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested does not exist: {}",
                    std::any::type_name::<T>()
                )
            })
            .as_ptr()
            .cast()
    }

    unsafe fn apply_tls_snapshot(&snap: &Self::TlsSnapshot) {
        unsafe { T::tls().set(snap) }
    }
}

unsafe impl RandomComponentList for () {
    type Tokens = ();
    type TokensMut = ();
    type ParamState = ();
    type TlsSnapshot = ();

    fn get_param_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::ParamState {}

    fn update_access_sets(
        _state: &Self::ParamState,
        _world: &mut World,
        _system_meta: &mut SystemMeta,
    ) {
    }

    fn fetch_tls_snapshot() -> Self::TlsSnapshot {}

    unsafe fn tls_snapshot_from_world(
        _state: &Self::ParamState,
        _world: UnsafeWorldCell<'_>,
    ) -> Self::TlsSnapshot {
    }

    unsafe fn apply_tls_snapshot(_snap: &Self::TlsSnapshot) {}
}

macro_rules! impl_random_component_list {
    () => {};
    ($first:ident $($rest:ident)*) => {
        unsafe impl<$first: RandomComponentList $(, $rest: RandomComponentList)*> RandomComponentList for ($first, $($rest,)*) {
            type Tokens = ($first::Tokens, $($rest::Tokens,)*);
            type TokensMut = ($first::TokensMut, $($rest::TokensMut,)*);
            type ParamState = ($first::ParamState, $($rest::ParamState,)*);
            type TlsSnapshot = ($first::TlsSnapshot, $($rest::TlsSnapshot,)*);

            fn get_param_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::ParamState {
                ($first::get_param_state(world, system_meta), $($rest::get_param_state(world, system_meta),)*)
            }

            #[allow(non_snake_case)]
            fn update_access_sets(
                ($first, $($rest,)*): &Self::ParamState,
                world: &mut World,
                system_meta: &mut SystemMeta,
            ) {
                $first::update_access_sets($first, world, system_meta);
                $($rest::update_access_sets($rest, world, system_meta);)*
            }

            fn fetch_tls_snapshot() -> Self::TlsSnapshot {
                ($first::fetch_tls_snapshot(), $($rest::fetch_tls_snapshot(),)*)
            }

            #[allow(non_snake_case)]
            unsafe fn tls_snapshot_from_world(($first, $($rest,)*): &Self::ParamState, world: UnsafeWorldCell<'_>,) -> Self::TlsSnapshot {
                ($first::tls_snapshot_from_world($first, world), $($rest::tls_snapshot_from_world($rest, world),)*)
            }

            #[allow(non_snake_case)]
            unsafe fn apply_tls_snapshot(($first, $($rest,)*): &Self::TlsSnapshot) {
                $first::apply_tls_snapshot($first);
                $($rest::apply_tls_snapshot($rest);)*
            }
        }

        impl_random_component_list!($($rest)*);
    };
}

impl_random_component_list!(T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12);

// === RandomComponent === //

pub struct RandomComponentToken<T> {
    _ty: PhantomData<fn() -> T>,
}

pub unsafe trait RandomComponent: 'static + Sized + Send + Sync {
    unsafe fn tls() -> &'static LocalKey<Cell<*mut RandomArena<Self>>>;

    fn arena<'a>() -> &'a RandomArena<Self> {
        autoken::tie!('a => ref RandomComponentToken<Self>);
        unsafe { &*Self::tls().get() }
    }

    fn arena_mut<'a>() -> &'a mut RandomArena<Self> {
        autoken::tie!('a => mut RandomComponentToken<Self>);
        unsafe { &mut *Self::tls().get() }
    }
}

#[doc(hidden)]
pub mod random_component_internals {
    pub use {
        super::{RandomArena, RandomComponent},
        std::{cell::Cell, ptr::null_mut, thread::LocalKey, thread_local},
    };
}

#[macro_export]
macro_rules! random_component {
    ($($ty:ty),*$(,)?) => {$(
        unsafe impl $crate::util::arena::random_component_internals::RandomComponent for $ty {
            unsafe fn tls() -> &'static $crate::util::arena::random_component_internals::LocalKey<
                $crate::util::arena::random_component_internals::Cell<
                    *mut $crate::util::arena::random_component_internals::RandomArena<Self>,
                >>
            {
                $crate::util::arena::random_component_internals::thread_local! {
                    static TLS: $crate::util::arena::random_component_internals::Cell<
                        *mut $crate::util::arena::random_component_internals::RandomArena<$ty>,
                    > = const {
                        $crate::util::arena::random_component_internals::Cell::new(
                            $crate::util::arena::random_component_internals::null_mut(),
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
        f.debug_tuple("Obj")
            .field(&self.index.into_raw_parts().0)
            .field(&self.index.into_raw_parts().1)
            .finish()
    }
}

impl<T> Copy for Obj<T> {}

impl<T> Clone for Obj<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: RandomComponent> Obj<T> {
    pub fn new(value: T) -> Self {
        Self::from_index(T::arena_mut().arena.insert(value))
    }

    pub fn destroy(me: Self) {
        T::arena_mut().arena.remove(me.index);
    }

    #[allow(clippy::should_implement_trait)]
    pub fn deref<'a>(self) -> &'a T {
        autoken::tie!('a => ref RandomComponentToken<T>);
        &T::arena().arena[self.index]
    }

    #[allow(clippy::should_implement_trait)]
    pub fn deref_mut<'a>(self) -> &'a mut T {
        autoken::tie!('a => mut RandomComponentToken<T>);
        &mut T::arena_mut().arena[self.index]
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

impl<T: RandomComponent> Deref for Obj<T> {
    type Target = T;

    fn deref<'a>(&'a self) -> &'a Self::Target {
        autoken::tie!('a => ref RandomComponentToken<T>);
        (*self).deref()
    }
}

impl<T: RandomComponent> DerefMut for Obj<T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut Self::Target {
        autoken::tie!('a => mut RandomComponentToken<T>);
        (*self).deref_mut()
    }
}

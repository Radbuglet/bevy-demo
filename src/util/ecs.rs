#![allow(clippy::missing_safety_doc)]

use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::QueryEntityError,
    system::{Query, SystemParam},
    world::Mut,
};

// === AnyMonoQuery === //

pub struct AnyMonoQuery<'world, 'state, T: Component> {
    immutable: Query<'world, 'state, &'static T>,
    mutable: Option<Query<'world, 'state, &'static mut T>>,
}

impl<'world, 'state, T: Component> AnyMonoQuery<'world, 'state, T> {
    pub fn new_ref(immutable: Query<'world, 'state, &'static T>) -> Self {
        Self {
            immutable,
            mutable: None,
        }
    }

    pub fn new_mut(mutable: Query<'world, 'state, &'static mut T>) -> Self {
        let immutable: Query<'_, '_, &'static T> = mutable.to_readonly();
        let immutable: Query<'world, 'state, &'static T> = unsafe {
            // Safety: it is safe (albeit potentially unsound) to use `immutable` and `mutable` queries
            // at the same time since the only thing we're prolonging is the lifetime of the immutable
            // reference to the queries' shared `state` object and the immutable reference to the
            // shared world object.
            std::mem::transmute(immutable)
        };

        Self {
            immutable,
            mutable: Some(mutable),
        }
    }

    pub fn get(&self, entity: Entity) -> Result<&T, QueryEntityError> {
        self.immutable.get(entity)
    }

    pub unsafe fn get_mut(&mut self, entity: Entity) -> Result<&mut T, QueryEntityError> {
        self.mutable
            .as_mut()
            .unwrap_unchecked()
            .get_mut(entity)
            .map(Mut::into_inner)
    }
}

// === RandomComponent === //

pub trait RandomComponent: Component {
    #[allow(opaque_hidden_inferred_bound)]
    fn cap() -> impl AnyMonoQueryCapHelper<Component = Self>;
}

pub trait AnyMonoQueryCapHelper: Sized {
    type Component: Component;

    unsafe fn provide_ref<R>(
        &self,
        query: &mut AnyMonoQuery<'_, '_, Self::Component>,
        f: impl FnOnce() -> R,
    ) -> R;

    unsafe fn provide_mut<R>(
        &self,
        query: &mut AnyMonoQuery<'_, '_, Self::Component>,
        f: impl FnOnce() -> R,
    ) -> R;

    fn get<'a>(&self, entity: Entity) -> Result<&'a Self::Component, QueryEntityError>;

    fn get_mut<'a>(&self, entity: Entity) -> Result<&'a mut Self::Component, QueryEntityError>;
}

#[doc(hidden)]
pub mod random_component_internals {
    pub use {
        super::{AnyMonoQuery, AnyMonoQueryCapHelper, RandomComponent},
        autoken::{cap, tie, CapTarget},
        bevy_ecs::{entity::Entity, query::QueryEntityError},
        std::{ops::FnOnce, result::Result},
    };
}

#[macro_export]
macro_rules! random_component {
    ($($ty:ty),*$(,)?) => {$(
        impl $crate::util::ecs::random_component_internals::RandomComponent for $ty {
            fn cap() -> impl $crate::util::ecs::random_component_internals::AnyMonoQueryCapHelper<Component = Self> {
                $crate::util::ecs::random_component_internals::cap! {
                    Cap<'world, 'state> = $crate::util::ecs::random_component_internals::AnyMonoQuery<'_, '_, $ty>;
                }

                impl $crate::util::ecs::random_component_internals::AnyMonoQueryCapHelper for Cap {
                    type Component = $ty;

                    unsafe fn provide_ref<R>(
                        &self,
                        query: &mut $crate::util::ecs::random_component_internals::AnyMonoQuery<'_, '_, Self::Component>,
                        f: impl $crate::util::ecs::random_component_internals::FnOnce() -> R,
                    ) -> R {
                        <Cap as $crate::util::ecs::random_component_internals::CapTarget<_>>::provide(query, f)
                    }

                    unsafe fn provide_mut<R>(
                        &self,
                        query: &mut $crate::util::ecs::random_component_internals::AnyMonoQuery<'_, '_, Self::Component>,
                        f: impl $crate::util::ecs::random_component_internals::FnOnce() -> R,
                    ) -> R {
                        <Cap as $crate::util::ecs::random_component_internals::CapTarget<_>>::provide(query, f)
                    }

                    fn get<'a>(
                        &self,
                        entity: $crate::util::ecs::random_component_internals::Entity
                    ) -> $crate::util::ecs::random_component_internals::Result<
                        &'a Self::Component,
                        $crate::util::ecs::random_component_internals::QueryEntityError,
                    > {
                        $crate::util::ecs::random_component_internals::tie!('a => ref Cap);
                        Cap::get(|v| v.get(entity)).0
                    }

                    fn get_mut<'a>(
                        &self,
                        entity: $crate::util::ecs::random_component_internals::Entity,
                    ) -> $crate::util::ecs::random_component_internals::Result<
                        &'a mut Self::Component,
                        $crate::util::ecs::random_component_internals::QueryEntityError,
                    > {
                        $crate::util::ecs::random_component_internals::tie!('a => mut Cap);
                        unsafe { Cap::get_mut(|v| v.get_mut(entity)).0 }
                    }
                }

                Cap
            }
        }
    )*};
}

// === ComponentSet === //

pub type RandomAccess<'world, 'state, S> = <S as ComponentSet>::Query<'world, 'state>;

pub trait ComponentSet {
    type Query<'world, 'state>: RandomQuery;
}

pub trait RandomQuery: SystemParam {
    fn provide<R>(self, f: impl FnOnce() -> R) -> R;
}

impl<'world, 'state, T: RandomComponent> RandomQuery for Query<'world, 'state, &'static T> {
    fn provide<R>(self, f: impl FnOnce() -> R) -> R {
        unsafe { T::cap().provide_ref(&mut AnyMonoQuery::new_ref(self), f) }
    }
}

impl<T: RandomComponent> ComponentSet for &'static T {
    type Query<'world, 'state> = Query<'world, 'state, &'static T>;
}

impl<'world, 'state, T: RandomComponent> RandomQuery for Query<'world, 'state, &'static mut T> {
    fn provide<R>(self, f: impl FnOnce() -> R) -> R {
        unsafe { T::cap().provide_mut(&mut AnyMonoQuery::new_mut(self), f) }
    }
}

impl<T: RandomComponent> ComponentSet for &'static mut T {
    type Query<'world, 'state> = Query<'world, 'state, &'static mut T>;
}

macro_rules! impl_component_set {
    () => {};
    ($first:ident $($remaining:ident)*) => {
        impl<$first: RandomQuery, $($remaining: RandomQuery,)*> RandomQuery for ($first, $($remaining,)*) {
            #[allow(non_snake_case)]
            fn provide<R>(self, f: impl FnOnce() -> R) -> R {
                let ($first, $($remaining,)*) = self;
                let f = move || $first.provide(f);
                $( let f = move || $remaining.provide(f); )*

                f()
            }
        }

        impl<$first: ComponentSet, $($remaining: ComponentSet,)*> ComponentSet for ($first, $($remaining,)*) {
            type Query<'world, 'state> = ($first::Query<'world, 'state>, $($remaining::Query<'world, 'state>,)*);
        }

        impl_component_set!($($remaining)*);
    };
}

impl_component_set!(T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12 T13 T14 T15 T16);

// === RandomEntityExt === //

pub trait RandomEntityExt: Sized {
    fn try_get<'a, T: RandomComponent>(self) -> Result<&'a T, QueryEntityError>;

    fn try_get_mut<'a, T: RandomComponent>(self) -> Result<&'a mut T, QueryEntityError>;

    fn get<'a, T: RandomComponent>(self) -> &'a T;

    fn get_mut<'a, T: RandomComponent>(self) -> &'a mut T;
}

impl RandomEntityExt for Entity {
    fn try_get<'a, T: RandomComponent>(self) -> Result<&'a T, QueryEntityError> {
        T::cap().get(self)
    }

    fn try_get_mut<'a, T: RandomComponent>(self) -> Result<&'a mut T, QueryEntityError> {
        T::cap().get_mut(self)
    }

    fn get<'a, T: RandomComponent>(self) -> &'a T {
        self.try_get::<T>().unwrap()
    }

    fn get_mut<'a, T: RandomComponent>(self) -> &'a mut T {
        self.try_get_mut::<T>().unwrap()
    }
}

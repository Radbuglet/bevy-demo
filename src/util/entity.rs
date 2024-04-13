use std::any::TypeId;

use autoken::cap;
use generational_arena::{Arena, Index};
use rustc_hash::FxHashMap;

use super::arena::{Component, Obj};

cap! {
    pub EntityManagerCap = EntityManager;
}

#[derive(Debug, Default)]
pub struct EntityManager {
    entities: Arena<()>,
    comp_maps: FxHashMap<(TypeId, Index), Index>,
}

#[derive(Debug, Copy, Hash, Eq, PartialEq, Clone)]
pub struct Entity {
    index: Index,
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity {
    pub fn new() -> Self {
        let entity_mgr = EntityManagerCap::get_mut(|v| v).0;

        Self {
            index: entity_mgr.entities.insert(()),
        }
    }

    pub fn insert<T: Component>(self, value: Obj<T>) {
        EntityManagerCap::get_mut(|v| v)
            .0
            .comp_maps
            .insert((TypeId::of::<T>(), self.index), value.index());
    }

    pub fn get<T: Component>(self) -> Obj<T> {
        Obj::from_index(EntityManagerCap::get(|v| v).0.comp_maps[&(TypeId::of::<T>(), self.index)])
    }

    pub fn with<T: Component>(self, value: T) -> Self {
        self.insert(Obj::new(value));
        self
    }

    pub fn destroy(self) {
        EntityManagerCap::get_mut(|v| v)
            .0
            .entities
            .remove(self.index);

        // TODO: Remove component entries
    }
}

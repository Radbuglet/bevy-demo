use std::fmt;

use bevy_ecs::entity::Entity;
use rustc_hash::FxHashMap;

use crate::{
    random_component,
    util::{
        arena::{Obj, RandomComponent, RandomEntityExt},
        lang::ensure_index,
    },
};

random_component!(MaterialRegistry, BaseMaterialDescriptor);

// === MaterialRegistry === //

#[derive(Debug, Default)]
pub struct MaterialRegistry {
    name_map: FxHashMap<String, MaterialId>,
    descriptors: Vec<Entity>,
}

impl MaterialRegistry {
    pub fn register(&mut self, name: impl Into<String>, entity: Entity) -> MaterialId {
        let name = name.into();
        let did = MaterialId(self.descriptors.len() as u16);
        self.name_map.insert(name.clone(), did);
        self.descriptors.push(entity);
        entity.insert(BaseMaterialDescriptor { id: did, name });
        did
    }

    pub fn lookup(&self, id: MaterialId) -> Entity {
        self.descriptors[id.0 as usize]
    }

    pub fn lookup_by_name(&self, name: &str) -> Option<MaterialId> {
        self.name_map.get(name).copied()
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct MaterialId(pub u16);

impl MaterialId {
    pub const AIR: Self = MaterialId(0);
}

#[derive(Debug)]
pub struct BaseMaterialDescriptor {
    pub id: MaterialId,
    pub name: String,
}

pub struct MaterialCache<T> {
    cache: Vec<Option<Obj<T>>>,
}

impl<T> fmt::Debug for MaterialCache<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialDescriptorCache")
            .finish_non_exhaustive()
    }
}

impl<T> Default for MaterialCache<T> {
    fn default() -> Self {
        Self { cache: Vec::new() }
    }
}

impl<T> MaterialCache<T> {
    pub fn get(&mut self, registry: &MaterialRegistry, id: MaterialId) -> Option<Obj<T>>
    where
        T: RandomComponent,
    {
        if let Some(Some(cached)) = self.cache.get(id.0 as usize) {
            return Some(*cached);
        }

        let obj = registry.lookup(id).try_get::<T>();
        *ensure_index(&mut self.cache, id.0 as usize) = obj;
        obj
    }
}

use bevy_ecs::{event::EventReader, system::Query};

use crate::{
    game::math::aabb::Aabb,
    random_component,
    util::arena::{Obj, ObjOwner, RandomAccess, RandomEntityExt},
};

use super::data::{TileWorld, WorldCreatedChunk};

// === ChunkColliders === //

random_component!(ChunkColliders, Collider);

#[derive(Debug, Default)]
pub struct ChunkColliders {
    aabbs: Vec<Aabb>,
    handles: Vec<Obj<Collider>>,
}

#[derive(Debug, Default)]
pub struct Collider {
    index: usize,
}

impl ChunkColliders {
    pub fn register(&mut self, mut collider: Obj<Collider>, aabb: Aabb) {
        collider.index = self.handles.len();
        self.aabbs.push(aabb);
        self.handles.push(collider);
    }

    pub fn unregister(&mut self, collider: Obj<Collider>) {
        self.aabbs.swap_remove(collider.index);
        self.handles.swap_remove(collider.index);

        if let Some(moved) = self.handles.get(collider.index) {
            moved.deref_mut().index = collider.index;
        }
    }

    pub fn set_aabb(&mut self, collider: Obj<Collider>, aabb: Aabb) {
        self.aabbs[collider.index] = aabb;
    }

    pub fn aabbs(&self) -> impl ExactSizeIterator<Item = Aabb> + '_ {
        self.aabbs.iter().copied()
    }
}

// === Systems === //

pub fn build(app: &mut crate::AppBuilder) {
    app.add_unlinker::<ChunkColliders>();
    app.add_unlinker::<Collider>();

    app.update.add_systems(sys_add_collider_to_chunk);
}

pub fn sys_add_collider_to_chunk(
    mut events: EventReader<WorldCreatedChunk>,
    mut rand: RandomAccess<&mut ChunkColliders>,
    query: Query<(&ObjOwner<TileWorld>,)>,
) {
    rand.provide(|| {
        for &WorldCreatedChunk { chunk, .. } in events.read().filter(|e| query.contains(e.world)) {
            chunk.insert(ChunkColliders::default());
        }
    });
}

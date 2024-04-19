use std::ops::ControlFlow;

use bevy_app::{App, PostUpdate, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    query::{Added, Changed},
    removal_detection::RemovedComponents,
    schedule::IntoSystemConfigs,
    system::Query,
};
use macroquad::math::{IVec2, Vec2};
use rustc_hash::FxHashSet;

use crate::{
    game::math::aabb::Aabb,
    random_component,
    util::arena::{Obj, ObjOwner, RandomAccess, RandomAppExt, RandomEntityExt, SendsEvent},
};

use super::data::{TileChunk, TileLayerConfig, TileWorld, WorldCreatedChunk};

random_component!(WorldColliders, TrackedColliderChunk, TrackedCollider);

// === Collider === //

#[derive(Debug, Component)]
pub struct InsideWorld(pub Obj<TileWorld>);

#[derive(Debug, Component)]
pub struct Collider(pub Aabb);

// === WorldCollisions === //

#[derive(Debug)]
pub struct WorldColliders {
    data: Obj<TileWorld>,
}

impl WorldColliders {
    pub fn new(data: Obj<TileWorld>) -> Self {
        Self { data }
    }

    pub fn collisions<B>(
        &self,
        aabb: Aabb,
        mut f: impl FnMut((Entity, Aabb)) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        let config = self.data.config();

        let mut chunks = FxHashSet::default();

        for chunk in config
            .actor_aabb_to_tile(aabb.grow(Vec2::splat(10.)))
            .inclusive()
            .iter()
        {
            chunks.insert(TileLayerConfig::decompose_world_pos(chunk).0);
        }

        for &chunk in &chunks {
            let chunk =
                get_collider_chunk_or_insert(self.data, self.data.chunk_or_create(chunk).entity());

            for isect in chunk.intersections(aabb) {
                f(isect)?;
            }
        }

        ControlFlow::Continue(())
    }
}

// === ChunkColliders === //

#[derive(Debug)]
pub struct TrackedColliderChunk {
    world: Obj<TileWorld>,
    config: TileLayerConfig,
    pos: IVec2,

    aabbs: Vec<Aabb>,
    handles: Vec<Obj<TrackedCollider>>,
}

#[derive(Debug)]
pub struct TrackedCollider {
    chunk: Obj<TrackedColliderChunk>,
    index: usize,
}

impl TrackedColliderChunk {
    pub fn register(mut self: Obj<Self>, mut collider: Obj<TrackedCollider>, aabb: Aabb) {
        collider.chunk = self;
        collider.index = self.handles.len();
        self.aabbs.push(aabb);
        self.handles.push(collider);
    }

    pub fn unregister(mut self: Obj<Self>, collider: Obj<TrackedCollider>) {
        self.aabbs.swap_remove(collider.index);
        self.handles.swap_remove(collider.index);

        if let Some(moved) = self.handles.get(collider.index) {
            moved.deref_mut().index = collider.index;
        }
    }

    pub fn set_aabb(&mut self, collider: Obj<TrackedCollider>, aabb: Aabb) {
        self.aabbs[collider.index] = aabb;
    }

    pub fn aabbs(&self) -> impl ExactSizeIterator<Item = (Entity, Aabb)> + '_ {
        self.handles
            .iter()
            .copied()
            .map(Obj::entity)
            .zip(self.aabbs.iter().copied())
    }

    pub fn intersections(&self, aabb: Aabb) -> impl Iterator<Item = (Entity, Aabb)> + '_ {
        self.aabbs()
            .filter(move |&(_, other)| aabb.intersects(other))
    }
}

// === Systems === //

pub fn plugin(app: &mut App) {
    app.add_random_component::<WorldColliders>();
    app.add_random_component::<TrackedColliderChunk>();
    app.add_random_component::<TrackedCollider>();

    app.add_systems(
        Update,
        (
            sys_add_collider_to_chunk,
            sys_add_collider,
            sys_move_colliders.after(sys_add_collider),
        ),
    );

    app.add_systems(PostUpdate, (sys_remove_collider,));
}

pub fn sys_add_collider(
    mut rand: RandomAccess<(
        &mut TrackedColliderChunk,
        &mut TrackedCollider,
        &mut TileWorld,
        &mut TileChunk,
        SendsEvent<WorldCreatedChunk>,
    )>,
    mut query: Query<(Entity, &Collider, &InsideWorld), Added<Collider>>,
) {
    rand.provide(|| {
        for (entity, &Collider(aabb), &InsideWorld(world)) in query.iter_mut() {
            let chunk = world.chunk_or_create(world.config().actor_to_decomposed(aabb.center()).0);
            let chunk = get_collider_chunk_or_insert(world, chunk.entity());

            let obj = entity.insert(TrackedCollider { chunk, index: 0 });
            chunk.register(obj, aabb);
        }
    });
}

pub fn sys_move_colliders(
    mut rand: RandomAccess<(
        &mut TrackedColliderChunk,
        &mut TrackedCollider,
        &mut TileWorld,
        &mut TileChunk,
        SendsEvent<WorldCreatedChunk>,
    )>,
    mut query: Query<(&Collider, &ObjOwner<TrackedCollider>), Changed<Collider>>,
) {
    rand.provide(|| {
        for (&Collider(aabb), &ObjOwner(tracked)) in query.iter_mut() {
            // Ensure that we moved to a new chunk
            let old_chunk = tracked.chunk;
            let config = old_chunk.config;
            let world = old_chunk.world;
            let old_pos = old_chunk.pos;

            let new_pos_world = aabb.center();
            let new_pos = config.actor_to_decomposed(new_pos_world).0;

            if new_pos == old_pos {
                old_chunk.deref_mut().aabbs[tracked.index] = aabb;
            } else {
                // Remove from the previous chunk
                old_chunk.unregister(tracked);

                // Move them to a new chunk
                let new_chunk = world.chunk_or_create(new_pos).entity();
                let new_chunk = get_collider_chunk_or_insert(world, new_chunk);

                new_chunk.register(tracked, aabb);
            }
        }
    });
}

pub fn sys_remove_collider(
    mut removed: RemovedComponents<ObjOwner<TrackedCollider>>,
    mut rand: RandomAccess<(&mut TrackedColliderChunk, &mut TrackedCollider)>,
) {
    rand.provide(|| {
        for collider in removed.read() {
            let collider = collider.get::<TrackedCollider>();
            collider.chunk.unregister(collider);
        }
    });
}

pub fn sys_add_collider_to_chunk(
    mut events: EventReader<WorldCreatedChunk>,
    mut rand: RandomAccess<(&TileWorld, &TileChunk, &mut TrackedColliderChunk)>,
    query: Query<(&ObjOwner<TileWorld>,)>,
) {
    rand.provide(|| {
        let events = events.read().filter(|e| query.contains(e.world));

        for &WorldCreatedChunk { world, chunk } in events {
            get_collider_chunk_or_insert(world.get::<TileWorld>(), chunk);
        }
    });
}

pub fn get_collider_chunk_or_insert(
    world: Obj<TileWorld>,
    chunk: Entity,
) -> Obj<TrackedColliderChunk> {
    chunk.try_get::<TrackedColliderChunk>().unwrap_or_else(|| {
        chunk.insert(TrackedColliderChunk {
            world,
            pos: chunk.get::<TileChunk>().pos(),
            config: world.config(),
            aabbs: Vec::new(),
            handles: Vec::new(),
        })
    })
}

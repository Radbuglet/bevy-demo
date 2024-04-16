use bevy_ecs::{removal_detection::RemovedComponents, system::Query};
use macroquad::math::IVec2;
use rustc_hash::FxHashMap;

use crate::{
    random_component,
    util::arena::{despawn_entity, spawn_entity, Obj, ObjOwner, RandomAccess, RandomEntityExt},
};

random_component!(WorldTile, ChunkTile);

#[derive(Debug, Default)]
pub struct WorldTile {
    chunks: FxHashMap<IVec2, Obj<ChunkTile>>,
}

impl WorldTile {
    pub fn add(mut self: Obj<Self>, pos: IVec2, mut chunk: Obj<ChunkTile>) {
        chunk.world = Some(self);
        chunk.pos = pos;
        chunk.neighbors[3] = Some(chunk);
        self.chunks.insert(pos, chunk);
    }
}

#[derive(Debug)]
pub struct ChunkTile {
    world: Option<Obj<WorldTile>>,
    neighbors: [Option<Obj<ChunkTile>>; 4],
    pos: IVec2,
    tiles: Box<[u16; 16 * 16]>,
}

impl Default for ChunkTile {
    fn default() -> Self {
        Self {
            world: None,
            neighbors: [None; 4],
            pos: IVec2::ZERO,
            tiles: Box::new([0; 256]),
        }
    }
}

impl ChunkTile {
    pub fn unlink(mut self: Obj<Self>) {
        let Some(mut world) = self.world else {
            return;
        };

        self.world = None;
        world.chunks.remove(&self.pos);
    }
}

// === Systems === //

pub fn build(app: &mut crate::AppBuilder) {
    app.startup.add_systems(system_build_world);
    app.update.add_systems(system_test_world);
    app.disposer.add_systems(system_clean_up_dead_chunks);
    app.add_unlinker::<WorldTile>();
    app.add_unlinker::<ChunkTile>();
}

fn system_build_world(mut rand: RandomAccess<(&mut WorldTile, &mut ChunkTile)>) {
    rand.provide(|| {
        let world = spawn_entity(());
        let world_data = world.insert(WorldTile::default());

        let chunk = spawn_entity(());
        world_data.add(IVec2::ZERO, chunk.insert(ChunkTile::default()));

        despawn_entity(chunk);
    });
}

fn system_clean_up_dead_chunks(
    mut query: RemovedComponents<ObjOwner<ChunkTile>>,
    mut rand: RandomAccess<(&mut WorldTile, &mut ChunkTile)>,
) {
    rand.provide(|| {
        for entity in query.read() {
            entity.get::<ChunkTile>().unlink();
        }
    });
}

fn system_test_world(
    mut query: Query<&ObjOwner<WorldTile>>,
    mut rand: RandomAccess<(&mut WorldTile, &mut ChunkTile)>,
) {
    rand.provide(|| {
        for &ObjOwner(world) in query.iter_mut() {
            for val in world.chunks.values() {
                dbg!(val, Obj::is_alive(*val));
            }
        }
    });
}

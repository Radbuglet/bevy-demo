use bevy_ecs::system::{Commands, Query};
use macroquad::math::IVec2;
use rustc_hash::FxHashMap;

use crate::{
    random_component,
    util::arena::{Obj, ObjOwner, RandomAccess, RandomEntityExt},
};

#[derive(Debug, Default)]
pub struct WorldTile {
    tiles: FxHashMap<IVec2, Obj<ChunkTile>>,
}

impl WorldTile {
    pub fn add(&mut self, pos: IVec2, mut chunk: Obj<ChunkTile>) {
        chunk.pos = pos;
        chunk.neighbors[3] = Some(chunk);
    }
}

#[derive(Debug)]
pub struct ChunkTile {
    pos: IVec2,
    tiles: Box<[u16; 16 * 16]>,
    neighbors: [Option<Obj<ChunkTile>>; 4],
}

impl Default for ChunkTile {
    fn default() -> Self {
        Self {
            pos: IVec2::ZERO,
            tiles: Box::new([0; 256]),
            neighbors: [None; 4],
        }
    }
}

random_component!(WorldTile, ChunkTile);

// === Systems === //

pub fn build(app: &mut crate::AppBuilder) {
    app.startup.add_systems(system_build_world);
    app.update.add_systems(system_test_world);
    app.add_unlinker::<WorldTile>();
    app.add_unlinker::<ChunkTile>();
}

fn system_build_world(mut cmd: Commands, mut rand: RandomAccess<(&mut WorldTile, &mut ChunkTile)>) {
    rand.provide(|| {
        let world = cmd.spawn(()).id();
        let mut world_data = world.insert(WorldTile::default());

        let chunk = cmd.spawn(()).id();
        world_data.add(IVec2::ZERO, chunk.insert(ChunkTile::default()));
    });
}

fn system_test_world(
    mut query: Query<&ObjOwner<WorldTile>>,
    mut rand: RandomAccess<(&mut WorldTile, &mut ChunkTile)>,
) {
    rand.provide(|| {
        for &ObjOwner(world) in query.iter_mut() {
            dbg!(world);
            for val in world.tiles.values() {
                dbg!(val, Obj::is_alive(*val));
            }
        }
    });
}

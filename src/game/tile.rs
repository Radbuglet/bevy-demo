use bevy_ecs::system::Commands;
use macroquad::math::IVec2;
use rustc_hash::FxHashMap;

use crate::{
    random_component,
    util::arena::{Obj, RandomAccess, RandomEntityExt},
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

random_component!(WorldTile, ChunkTile);

// === Systems === //

pub fn build(app: &mut crate::AppBuilder) {
    app.startup.add_systems(system_build_world);
}

fn system_build_world(mut cmd: Commands, mut rand: RandomAccess<&mut WorldTile>) {
    rand.provide(|| {
        let mut world = cmd.spawn(()).id().insert(WorldTile::default());
    });
}

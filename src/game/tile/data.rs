use std::ops::ControlFlow;

use bevy_ecs::{entity::Entity, event::Event, removal_detection::RemovedComponents};
use macroquad::math::{IVec2, Vec2};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::{
    game::math::{
        aabb::{Aabb, AabbI},
        glam::{AaLine, Axis2, Sign, TileFace, Vec2Ext},
        scalar::ilerp_f32,
    },
    random_component, random_event,
    util::arena::{send_event, spawn_entity, Obj, ObjOwner, RandomAccess, RandomEntityExt},
};

use super::material::MaterialId;

// === Definition === //

random_component!(TileWorld, TileChunk);
random_event!(WorldCreatedChunk);

#[derive(Event)]
pub struct WorldCreatedChunk {
    pub world: Entity,
    pub chunk: Entity,
}

// === TileLayerConfig === //

#[derive(Debug, Copy, Clone)]
pub struct TileLayerConfig {
    pub size: f32,
    pub offset: Vec2,
}

impl TileLayerConfig {
    pub const CHUNK_EDGE: i32 = 16;
    pub const CHUNK_AREA: i32 = Self::CHUNK_EDGE * Self::CHUNK_EDGE;

    pub fn from_size(size: f32) -> Self {
        Self {
            size,
            offset: Vec2::ZERO,
        }
    }

    pub fn actor_to_tile_axis(&self, axis: Axis2, value: f32) -> i32 {
        let _ = axis;
        value.div_euclid(self.size).floor() as i32
    }

    pub fn actor_to_tile(&self, Vec2 { x, y }: Vec2) -> IVec2 {
        IVec2::new(
            self.actor_to_tile_axis(Axis2::X, x),
            self.actor_to_tile_axis(Axis2::Y, y),
        )
    }

    pub fn actor_aabb_to_tile(&self, aabb: Aabb) -> AabbI {
        AabbI {
            min: self.actor_to_tile(aabb.min),
            max: self.actor_to_tile(aabb.max),
        }
    }

    pub fn tile_to_actor_rect(&self, IVec2 { x, y }: IVec2) -> Aabb {
        Aabb::new_sized(
            Vec2::new(x as f32, y as f32) * self.size,
            Vec2::splat(self.size),
        )
    }

    pub fn floating_tile_to_actor_rect(&self, vec: Vec2) -> Aabb {
        Aabb::new_sized(vec * self.size, Vec2::splat(self.size))
    }

    pub fn decompose_world_pos(v: IVec2) -> (IVec2, IVec2) {
        let IVec2 { x, y } = v;

        (
            IVec2::new(
                x.div_euclid(Self::CHUNK_EDGE),
                y.div_euclid(Self::CHUNK_EDGE),
            ),
            IVec2::new(
                x.rem_euclid(Self::CHUNK_EDGE),
                y.rem_euclid(Self::CHUNK_EDGE),
            ),
        )
    }

    pub fn to_tile_index(v: IVec2) -> i32 {
        v.y * Self::CHUNK_EDGE + v.x
    }

    pub fn actor_to_decomposed(&self, actor: Vec2) -> (IVec2, IVec2) {
        Self::decompose_world_pos(self.actor_to_tile(actor))
    }

    pub fn tile_edge_line(&self, tile: IVec2, face: TileFace) -> AaLine {
        self.tile_to_actor_rect(tile).edge_line(face)
    }

    pub fn step_ray(&self, origin: Vec2, delta: Vec2) -> SmallVec<[RayIntersection; 2]> {
        let mut intersections = SmallVec::<[RayIntersection; 2]>::new();

        // Collect all possible intersections
        let origin_tile = self.actor_to_tile(origin);
        let dest = origin + delta;

        for axis in Axis2::iter() {
            let origin_value = origin.get_axis(axis);
            let delta_value = delta.get_axis(axis);
            let delta_sign = Sign::of_biased(delta_value);
            let dest_value = dest.get_axis(axis);

            // Ensure that we crossed a block boundary
            if self.actor_to_tile_axis(axis, origin_value)
                == self.actor_to_tile_axis(axis, dest_value)
            {
                continue;
            }

            // If we did, add a ray intersection
            let iface_value = self
                .tile_edge_line(origin_tile, TileFace::compose(axis, delta_sign))
                .norm;

            let isect_pos = origin.lerp(delta, ilerp_f32(origin_value, dest_value, iface_value));

            intersections.push(RayIntersection {
                face: TileFace::compose(axis, delta_sign),
                entered_tile: IVec2::ZERO,
                dist: origin.distance(isect_pos),
                isect_pos,
            });
        }

        // Sort them by distance
        intersections.sort_by(|a, b| a.dist.total_cmp(&b.dist));

        // Update tile positions
        let mut tile_pos = origin_tile;
        for intersection in &mut intersections {
            tile_pos += intersection.face.as_ivec();
            intersection.entered_tile = tile_pos;
        }

        intersections
    }

    pub fn step_ray_tiles<B>(
        &self,
        src: Vec2,
        dst: Vec2,
        mut f: impl FnMut(IVec2) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        let mut origin = src;
        let mut length = (dst - src).length();
        let delta = (dst - src) / length;

        if !delta.is_nan() {
            while length > 0. {
                let step_size = length.min(self.size);
                for isect in self.step_ray(origin, delta * step_size) {
                    f(isect.entered_tile)?;
                }
                length -= step_size;
                origin += delta * step_size;
            }
        }

        f(self.actor_to_tile(dst))?;

        ControlFlow::Continue(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RayIntersection {
    pub face: TileFace,
    pub entered_tile: IVec2,
    pub isect_pos: Vec2,
    pub dist: f32,
}

// === WorldTile === //

#[derive(Debug)]
pub struct TileWorld {
    config: TileLayerConfig,
    chunks: FxHashMap<IVec2, Obj<TileChunk>>,
}

impl TileWorld {
    pub fn new(config: TileLayerConfig) -> Self {
        Self {
            config,
            chunks: FxHashMap::default(),
        }
    }

    fn insert_chunk(mut self: Obj<Self>, pos: IVec2, mut chunk: Obj<TileChunk>) {
        chunk.world = Some(self);
        chunk.pos = pos;
        self.chunks.insert(pos, chunk);

        for face in TileFace::VARIANTS {
            let neighbor = self.chunks.get(&(pos + face.as_ivec())).copied();

            chunk.neighbors[face as usize] = neighbor;

            if let Some(mut neighbor) = neighbor {
                neighbor.neighbors[face.invert() as usize] = Some(chunk);
            }
        }
    }

    pub fn config(&self) -> TileLayerConfig {
        self.config
    }

    pub fn chunk_or_create(self: Obj<Self>, pos: IVec2) -> Obj<TileChunk> {
        if let Some(&chunk) = self.chunks.get(&pos) {
            return chunk;
        }

        let chunk = spawn_entity(());
        let chunk_obj = chunk.insert(TileChunk::default());
        self.insert_chunk(pos, chunk_obj);
        send_event(WorldCreatedChunk {
            world: self.entity(),
            chunk,
        });
        chunk_obj
    }

    pub fn tile(&self, pos: IVec2) -> MaterialId {
        let (chunk, block) = TileLayerConfig::decompose_world_pos(pos);
        self.chunks
            .get(&chunk)
            .map_or(MaterialId::AIR, |chunk| chunk.tile(block))
    }

    pub fn set_tile(self: Obj<Self>, pos: IVec2, data: MaterialId) {
        let (chunk, block) = TileLayerConfig::decompose_world_pos(pos);
        self.chunk_or_create(chunk).set_tile(block, data);
    }
}

// === TileChunk === //

#[derive(Debug)]
pub struct TileChunk {
    world: Option<Obj<TileWorld>>,
    neighbors: [Option<Obj<TileChunk>>; 4],
    pos: IVec2,
    tiles: Box<[u16; TileLayerConfig::CHUNK_AREA as usize]>,
}

impl Default for TileChunk {
    fn default() -> Self {
        Self {
            world: None,
            neighbors: [None; 4],
            pos: IVec2::ZERO,
            tiles: Box::new([0; TileLayerConfig::CHUNK_AREA as usize]),
        }
    }
}

impl TileChunk {
    pub fn pos(&self) -> IVec2 {
        self.pos
    }

    pub fn tile(&self, pos: IVec2) -> MaterialId {
        MaterialId(self.tiles[TileLayerConfig::to_tile_index(pos) as usize])
    }

    pub fn set_tile(&mut self, pos: IVec2, data: MaterialId) {
        self.tiles[TileLayerConfig::to_tile_index(pos) as usize] = data.0;
    }

    fn remove_from_world(mut self: Obj<Self>) {
        let Some(mut world) = self.world else {
            return;
        };

        self.world = None;
        world.chunks.remove(&self.pos);

        for (face, neighbor) in self.neighbors.into_iter().enumerate() {
            let face = TileFace::VARIANTS[face];
            let Some(mut neighbor) = neighbor else {
                continue;
            };

            neighbor.neighbors[face.invert() as usize] = None;
        }
    }
}

// === Systems === //

pub fn sys_unregister_chunk_from_world(
    mut query: RemovedComponents<ObjOwner<TileChunk>>,
    mut rand: RandomAccess<(&mut TileWorld, &mut TileChunk)>,
) {
    rand.provide(|| {
        for entity in query.read() {
            entity.get::<TileChunk>().remove_from_world();
        }
    });
}

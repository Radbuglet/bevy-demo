use std::ops::ControlFlow;

use bevy_ecs::entity::Entity;
use cbit::cbit;
use macroquad::math::{BVec2, IVec2, Vec2};
use smallvec::SmallVec;

use crate::{
    game::math::{
        aabb::Aabb,
        glam::{add_magnitude, Axis2, BVec2Ext, Sign, Vec2Ext},
    },
    random_component,
    util::arena::Obj,
};

use super::{
    collider::WorldColliders,
    data::TileWorld,
    material::{MaterialCache, MaterialId, MaterialRegistry},
};

random_component!(TileColliderDescriptor, KinematicApi);

// === TileColliderDescriptor === //

#[derive(Debug, Clone)]
pub struct TileColliderDescriptor {
    pub aabbs: SmallVec<[Aabb; 1]>,
}

impl TileColliderDescriptor {
    pub fn new(aabbs: impl IntoIterator<Item = Aabb>) -> Self {
        Self {
            aabbs: aabbs.into_iter().collect(),
        }
    }
}

// === AnyCollision === //

#[derive(Debug, Copy, Clone)]
pub enum AnyCollision {
    Tile(IVec2, MaterialId, Aabb),
    Collider(Entity, Aabb),
}

impl AnyCollision {
    pub fn aabb(self) -> Aabb {
        match self {
            AnyCollision::Tile(_, _, aabb) => aabb,
            AnyCollision::Collider(_, aabb) => aabb,
        }
    }
}

// === KinematicApi === //

#[derive(Debug)]
pub struct KinematicApi {
    data: Obj<TileWorld>,
    registry: Obj<MaterialRegistry>,
    colliders: Obj<WorldColliders>,
    cache: MaterialCache<TileColliderDescriptor>,
}

impl KinematicApi {
    pub const TOLERANCE: f32 = 0.01;

    pub fn new(
        data: Obj<TileWorld>,
        registry: Obj<MaterialRegistry>,
        colliders: Obj<WorldColliders>,
    ) -> Self {
        Self {
            data,
            registry,
            colliders,
            cache: MaterialCache::default(),
        }
    }

    pub fn iter_colliders_in<B>(
        &mut self,
        check_aabb: Aabb,
        mut f: impl FnMut(AnyCollision) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        let config = self.data.config();

        for tile in config.actor_aabb_to_tile(check_aabb).inclusive().iter() {
            let offset = config.tile_to_actor_rect(tile).min;
            let material = self.data.tile(tile);

            if material == MaterialId::AIR {
                continue;
            }

            let Some(colliders) = self.cache.get(&self.registry, material) else {
                continue;
            };

            for &tile_aabb in &colliders.aabbs {
                let tile_aabb = Aabb {
                    min: tile_aabb.min * config.size,
                    max: tile_aabb.max * config.size,
                };
                let tile_aabb = tile_aabb.translated(offset);

                if !tile_aabb.intersects(check_aabb) {
                    continue;
                }

                f(AnyCollision::Tile(tile, material, tile_aabb))?;
            }
        }

        cbit! {
            for (actor, collider) in self.colliders.collisions(check_aabb) {
                f(AnyCollision::Collider(actor, collider))?;
            }
        }

        ControlFlow::Continue(())
    }

    pub fn has_colliders_in(
        &mut self,
        check_aabb: Aabb,
        mut filter: impl FnMut(AnyCollision) -> bool,
    ) -> bool {
        cbit!(for collider in self.iter_colliders_in(check_aabb) {
            if filter(collider) {
                return true;
            }
        });

        false
    }

    pub fn get_clip_mask(
        &mut self,
        aabb: Aabb,
        by: Vec2,
        mut filter: impl FnMut(AnyCollision) -> bool,
    ) -> BVec2 {
        let mut mask = BVec2::default();

        for axis in Axis2::iter() {
            let signed_delta = by.get_axis(axis);
            let check_aabb =
                aabb.translate_extend(axis.unit_mag((Self::TOLERANCE * 2.).copysign(signed_delta)));

            mask.set_axis(axis, !self.has_colliders_in(check_aabb, &mut filter));
        }

        mask
    }

    pub fn move_by(
        &mut self,
        aabb: Aabb,
        by: Vec2,
        mut filter: impl FnMut(AnyCollision) -> bool,
    ) -> Vec2 {
        let mut aabb = aabb;
        let mut total_by = Vec2::ZERO;

        for axis in Axis2::iter() {
            let signed_delta = by.get_axis(axis);
            let check_aabb =
                aabb.translate_extend(axis.unit_mag(add_magnitude(signed_delta, Self::TOLERANCE)));

            let mut delta = signed_delta.abs();

            cbit!(for collider in self.iter_colliders_in(check_aabb) {
                let collider_aabb = collider.aabb();
                if !filter(collider) {
                    continue;
                }

                let acceptable_delta = if signed_delta < 0. {
                    // We're moving to the left/top so we're presumably right/below the target.
                    aabb.min.get_axis(axis) - collider_aabb.max.get_axis(axis)
                } else {
                    // We're moving to the right/bottom so we're presumably left/above the target.
                    collider_aabb.min.get_axis(axis) - aabb.max.get_axis(axis)
                }
                .abs();

                let acceptable_delta = acceptable_delta - Self::TOLERANCE;
                delta = delta.min(acceptable_delta.max(0.));
            });

            let delta = axis.unit_mag(Sign::of_biased(signed_delta).unit_mag(delta));

            total_by += delta;
            aabb = aabb.translated(delta);
        }

        total_by
    }
}

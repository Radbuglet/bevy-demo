use bevy_app::{App, Update};
use bevy_ecs::{component::Component, schedule::IntoSystemConfigs, system::Query};
use macroquad::math::Vec2;

use crate::{
    game::{
        math::{aabb::Aabb, glam::Vec2Ext},
        tile::{
            collider::{
                sys_add_collider, Collider, InsideWorld, TrackedCollider, TrackedColliderChunk,
                WorldColliders,
            },
            data::{TileChunk, TileWorld, WorldCreatedChunk},
            kinematic::{AnyCollision, KinematicApi, TileColliderDescriptor},
            material::MaterialRegistry,
        },
    },
    util::arena::{RandomAccess, RandomEntityExt, SendsEvent},
};

use super::player::sys_handle_controls;

// === Systems === //

#[derive(Component)]
pub struct Pos(pub Vec2);

#[derive(Component)]
pub struct Vel(pub Vec2);

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        sys_update_kinematics
            .after(sys_handle_controls)
            .before(sys_add_collider),
    );
}

pub fn sys_update_kinematics(
    mut query: Query<(&InsideWorld, &mut Pos, &mut Vel, &mut Collider)>,
    mut rand: RandomAccess<(
        &mut TileWorld,
        &mut TileChunk,
        &mut KinematicApi,
        &mut TrackedColliderChunk,
        &TrackedCollider,
        &WorldColliders,
        &TileColliderDescriptor,
        &MaterialRegistry,
        SendsEvent<WorldCreatedChunk>,
    )>,
) {
    rand.provide(|| {
        for (&InsideWorld(world), mut pos, mut vel, mut collider) in query.iter_mut() {
            let mut world = world.entity().get::<KinematicApi>();

            let delta = vel.0;
            let filter = |coll| match coll {
                AnyCollision::Tile(_, _, _) => true,
                AnyCollision::Collider(_, _) => false,
            };

            let delta = world.move_by(collider.0, delta, filter);
            pos.0 += delta;
            collider.0 = Aabb::new_centered(pos.0, Vec2::splat(40.));

            let mask = world.get_clip_mask(collider.0, vel.0, filter);
            vel.0 = vel.0.mask(mask);
        }
    });
}

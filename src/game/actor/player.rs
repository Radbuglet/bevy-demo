use std::collections::VecDeque;

use bevy_app::{App, Startup, Update};
use bevy_ecs::{component::Component, schedule::IntoSystemConfigs, system::Query};
use macroquad::{
    color::{Color, DARKPURPLE, GREEN, RED},
    input::{is_key_down, KeyCode},
    math::{IVec2, Vec2},
    shapes::draw_circle,
};

use crate::{
    game::{
        math::{aabb::Aabb, glam::Vec2Ext},
        tile::{
            collider::{
                sys_add_collider, Collider, InsideWorld, TrackedCollider, TrackedColliderChunk,
                WorldColliders,
            },
            data::{TileChunk, TileLayerConfig, TileWorld, WorldCreatedChunk},
            kinematic::{AnyCollision, KinematicApi, TileColliderDescriptor},
            material::{BaseMaterialDescriptor, MaterialRegistry},
            render::{RenderableWorld, SolidTileMaterial},
        },
    },
    util::arena::{spawn_entity, RandomAccess, RandomEntityExt, SendsEvent},
    Render,
};

#[derive(Component)]
pub struct Pos(Vec2);

#[derive(Component)]
pub struct Vel(Vec2);

#[derive(Component)]
pub struct Player {
    trail: VecDeque<Vec2>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, (sys_create_local_player,));

    app.add_systems(
        Update,
        (
            sys_handle_controls,
            sys_update_kinematics
                .after(sys_handle_controls)
                .before(sys_add_collider),
        ),
    );

    app.add_systems(Render, (sys_render_players,));
}

fn sys_create_local_player(
    mut rand: RandomAccess<(
        &mut TileWorld,
        &mut TileChunk,
        &mut MaterialRegistry,
        &mut BaseMaterialDescriptor,
        &mut TileColliderDescriptor,
        &mut SolidTileMaterial,
        &mut WorldColliders,
        &mut KinematicApi,
        SendsEvent<WorldCreatedChunk>,
    )>,
) {
    rand.provide(|| {
        let world = spawn_entity(RenderableWorld::default());
        let world_data = world.insert(TileWorld::new(TileLayerConfig {
            offset: Vec2::ZERO,
            size: 10.,
        }));
        let world_colliders = world.insert(WorldColliders::new(world_data));

        let mut registry = world.insert(MaterialRegistry::default());
        registry.register("game:air", spawn_entity(()));
        let grass = registry.register("game:grass", {
            let descriptor = spawn_entity(());
            descriptor.insert(SolidTileMaterial { color: GREEN });
            descriptor.insert(TileColliderDescriptor::new([Aabb::ZERO_TO_ONE]));
            descriptor
        });

        for x in 0..100 {
            world_data.set_tile(IVec2::new(x, x), grass);
        }

        world.insert(KinematicApi::new(world_data, registry, world_colliders));

        spawn_entity((
            Pos(Vec2::ZERO),
            Vel(Vec2::ONE),
            InsideWorld(world_data),
            Collider(Aabb::ZERO_TO_ONE),
            Player {
                trail: VecDeque::new(),
            },
        ));
    });
}

fn sys_update_kinematics(
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
            collider.0 = Aabb::new_centered(pos.0, Vec2::splat(50.));

            let mask = world.get_clip_mask(collider.0, vel.0, filter);
            vel.0 = vel.0.mask(mask);
        }
    });
}

fn sys_handle_controls(mut query: Query<(&Pos, &mut Vel, &mut Player)>) {
    let mut heading = Vec2::ZERO;
    if is_key_down(KeyCode::A) {
        heading += Vec2::NEG_X;
    }
    if is_key_down(KeyCode::D) {
        heading += Vec2::X;
    }
    if is_key_down(KeyCode::W) {
        heading += Vec2::NEG_Y;
    }
    if is_key_down(KeyCode::S) {
        heading += Vec2::Y;
    }

    heading = heading.normalize_or_zero();

    for (pos, mut vel, mut player) in query.iter_mut() {
        vel.0 += heading;
        vel.0 *= 0.98;

        player.trail.push_front(pos.0);
        if player.trail.len() > 100 {
            player.trail.pop_back();
        }
    }
}

fn sys_render_players(mut query: Query<(&Pos, &Player)>) {
    for (pos, player) in query.iter_mut() {
        for (i, &trail) in player.trail.iter().rev().enumerate() {
            draw_circle(
                trail.x,
                trail.y,
                20.,
                Color::from_vec(
                    DARKPURPLE
                        .to_vec()
                        .lerp(RED.to_vec(), i as f32 / player.trail.len() as f32),
                ),
            );
        }

        draw_circle(pos.0.x, pos.0.y, 20., RED);
    }
}

use std::collections::VecDeque;

use bevy_app::{App, Startup, Update};
use bevy_ecs::{
    component::Component,
    query::With,
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut},
};
use macroquad::{
    color::{Color, DARKPURPLE, GRAY, GREEN, RED},
    input::{is_key_down, KeyCode},
    math::{Affine2, IVec2, Vec2},
    shapes::draw_circle,
};

use crate::{
    game::{
        math::aabb::Aabb,
        tile::{
            collider::{Collider, InsideWorld, WorldColliders},
            data::{TileChunk, TileLayerConfig, TileWorld, WorldCreatedChunk},
            kinematic::{KinematicApi, TileColliderDescriptor},
            material::{BaseMaterialDescriptor, MaterialRegistry},
            render::{RenderableWorld, SolidTileMaterial},
        },
    },
    util::arena::{spawn_entity, RandomAccess, RandomEntityExt, SendsEvent},
    Render,
};

use super::{
    camera::{ActiveCamera, VirtualCamera, VirtualCameraConstraints},
    kinematic::{sys_update_kinematics, Pos, Vel},
};

// === Systems === //

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
            sys_focus_camera_on_player.after(sys_update_kinematics),
        ),
    );

    app.add_systems(Render, (sys_render_players,));
}

pub fn sys_create_local_player(
    mut rand: RandomAccess<(
        &mut TileWorld,
        &mut TileChunk,
        &mut MaterialRegistry,
        &mut BaseMaterialDescriptor,
        &mut TileColliderDescriptor,
        &mut SolidTileMaterial,
        &mut WorldColliders,
        &mut KinematicApi,
        &mut VirtualCamera,
        SendsEvent<WorldCreatedChunk>,
    )>,
    mut camera: ResMut<ActiveCamera>,
) {
    rand.provide(|| {
        // Spawn world
        let world = spawn_entity(RenderableWorld::default());

        // Setup camera
        camera.camera = Some(world.insert(VirtualCamera::new(
            Affine2::IDENTITY,
            Aabb::new_centered(Vec2::ZERO, Vec2::splat(1000.)),
            VirtualCameraConstraints::default().keep_visible_area(Vec2::new(1000., 1000.)),
        )));

        // Setup material registry
        let mut registry = world.insert(MaterialRegistry::default());
        registry.register("game:air", spawn_entity(()));
        let grass = registry.register("game:grass", {
            let descriptor = spawn_entity(());
            descriptor.insert(SolidTileMaterial { color: GREEN });
            descriptor.insert(TileColliderDescriptor::new([Aabb::ZERO_TO_ONE]));
            descriptor
        });
        let stone = registry.register("game:stone", {
            let descriptor = spawn_entity(());
            descriptor.insert(SolidTileMaterial { color: GRAY });
            descriptor.insert(TileColliderDescriptor::new([Aabb::ZERO_TO_ONE]));
            descriptor
        });

        // Setup world
        let world_data = world.insert(TileWorld::new(TileLayerConfig {
            offset: Vec2::ZERO,
            size: 50.,
        }));
        let world_colliders = world.insert(WorldColliders::new(world_data));

        for x in 0..500 {
            let v = (x as f32 / 10.).sin();
            world_data.set_tile(IVec2::new(x, (v * 10.) as i32), grass);
            world_data.set_tile(IVec2::new(x, (v * 10.) as i32 - 20), stone);
        }

        world.insert(KinematicApi::new(world_data, registry, world_colliders));

        // Spawn player
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

pub fn sys_handle_controls(
    mut rand: RandomAccess<(&TileWorld, &mut VirtualCamera)>,
    mut query: Query<(&Pos, &mut Vel, &mut Player)>,
) {
    rand.provide(|| {
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
    });
}

pub fn sys_focus_camera_on_player(
    mut query: Query<(&InsideWorld, &Pos), With<Player>>,
    mut rand: RandomAccess<(&mut TileWorld, &mut VirtualCamera)>,
) {
    rand.provide(|| {
        let Some((&InsideWorld(world), pos)) = query.iter_mut().next() else {
            return;
        };

        world
            .entity()
            .get::<VirtualCamera>()
            .set_transform(Affine2::from_translation(pos.0));
    });
}

pub fn sys_render_players(mut query: Query<(&Pos, &Player)>, camera: Res<ActiveCamera>) {
    let _guard = camera.apply();

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

use std::collections::VecDeque;

use bevy_app::{App, Startup, Update};
use bevy_ecs::{component::Component, schedule::IntoSystemConfigs, system::Query};
use macroquad::{
    color::{Color, DARKPURPLE, GREEN, RED},
    input::{is_key_down, KeyCode},
    math::Vec2,
    shapes::draw_circle,
    window::{screen_height, screen_width},
};

use crate::{
    game::{
        math::aabb::Aabb,
        tile::{
            collider::{sys_add_collider, Collider, InsideWorld},
            data::{TileChunk, TileLayerConfig, TileWorld, WorldCreatedChunk},
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
        &mut MaterialRegistry,
        &mut BaseMaterialDescriptor,
        &mut SolidTileMaterial,
    )>,
) {
    rand.provide(|| {
        let world = spawn_entity(RenderableWorld::default());
        let world_obj = world.insert(TileWorld::new(TileLayerConfig {
            offset: Vec2::ZERO,
            size: 10.,
        }));

        let mut registry = world.insert(MaterialRegistry::default());
        registry.register("game:air", spawn_entity(()));
        registry.register("game:grass", {
            let descriptor = spawn_entity(());
            descriptor.insert(SolidTileMaterial { color: GREEN });
            descriptor
        });

        spawn_entity((
            Pos(Vec2::ZERO),
            Vel(Vec2::ONE),
            InsideWorld(world_obj),
            Collider(Aabb::ZERO_TO_ONE),
            Player {
                trail: VecDeque::new(),
            },
        ));
    });
}

fn sys_update_kinematics(
    mut query: Query<(&InsideWorld, &mut Player, &mut Collider, &mut Pos, &mut Vel)>,
    mut rand: RandomAccess<(
        &mut TileWorld,
        &mut TileChunk,
        &mut MaterialRegistry,
        &mut BaseMaterialDescriptor,
        SendsEvent<WorldCreatedChunk>,
    )>,
) {
    rand.provide(|| {
        for (&InsideWorld(world_data), mut player, mut collider, mut pos, mut vel) in
            query.iter_mut()
        {
            pos.0 += vel.0;
            vel.0 *= 0.98;

            pos.0.x = pos.0.x.rem_euclid(screen_width());
            pos.0.y = pos.0.y.rem_euclid(screen_height());

            collider.0 = Aabb::new_centered(pos.0, Vec2::ONE);

            player.trail.push_front(pos.0);
            if player.trail.len() > 100 {
                player.trail.pop_back();
            }

            let world_mats = world_data.entity().get::<MaterialRegistry>();
            world_data.set_tile(
                world_data.config().actor_to_tile(pos.0),
                world_mats.lookup_by_name("game:grass").unwrap(),
            );
        }
    });
}

fn sys_handle_controls(mut query: Query<&mut Vel>) {
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

    for mut vel in query.iter_mut() {
        vel.0 += heading;
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

use std::collections::VecDeque;

use bevy_ecs::{
    component::Component,
    system::{Commands, Query},
};
use macroquad::{
    color::{Color, DARKPURPLE, RED},
    input::{is_key_down, KeyCode},
    math::Vec2,
    shapes::draw_circle,
    window::{screen_height, screen_width},
};

#[derive(Component)]
pub struct Pos(Vec2);

#[derive(Component)]
pub struct Vel(Vec2);

#[derive(Component, Default)]
pub struct Player {
    trail: VecDeque<Vec2>,
}

pub fn build(app: &mut crate::AppBuilder) {
    app.startup.add_systems(system_create_local_player);
    app.update.add_systems(system_update_kinematics);
    app.update.add_systems(system_handle_controls);
    app.render.add_systems(system_render_players);
}

fn system_create_local_player(mut cmd: Commands) {
    cmd.spawn((Pos(Vec2::ZERO), Vel(Vec2::ONE), Player::default()));
}

fn system_update_kinematics(mut query: Query<(&mut Pos, &mut Vel, &mut Player)>) {
    for (mut pos, mut vel, mut player) in query.iter_mut() {
        pos.0 += vel.0;
        vel.0 *= 0.98;

        pos.0.x = pos.0.x.rem_euclid(screen_width());
        pos.0.y = pos.0.y.rem_euclid(screen_height());

        player.trail.push_front(pos.0);
        if player.trail.len() > 100 {
            player.trail.pop_back();
        }
    }
}

fn system_handle_controls(mut query: Query<&mut Vel>) {
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

fn system_render_players(mut query: Query<(&Pos, &Player)>) {
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

use bevy_ecs::{
    component::Component,
    query::With,
    system::{Commands, Query},
};
use macroquad::{
    color::RED,
    input::{is_key_down, KeyCode},
    math::Vec2,
    shapes::draw_circle,
};

#[derive(Component)]
pub struct Pos(Vec2);

#[derive(Component)]
pub struct Vel(Vec2);

#[derive(Component)]
pub struct Player;

pub fn build(app: &mut crate::AppBuilder) {
    app.startup.add_systems(system_create_local_player);
    app.update.add_systems(system_update_kinematics);
    app.update.add_systems(system_handle_controls);
    app.render.add_systems(system_render_players);
}

fn system_create_local_player(mut cmd: Commands) {
    cmd.spawn((Pos(Vec2::ZERO), Vel(Vec2::ONE), Player));
}

fn system_update_kinematics(mut query: Query<(&mut Pos, &mut Vel)>) {
    for (mut pos, mut vel) in query.iter_mut() {
        pos.0 += vel.0;
        vel.0 *= 0.98;
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

fn system_render_players(mut query: Query<&Pos, With<Player>>) {
    for pos in query.iter_mut() {
        draw_circle(pos.0.x, pos.0.y, 20., RED);
    }
}

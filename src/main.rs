#![feature(arbitrary_self_types)]

use bevy_ecs::{schedule::Schedule, world::World};
use macroquad::{
    input::{is_key_pressed, is_quit_requested, KeyCode},
    window::next_frame,
};

pub mod game;
pub mod util;

#[derive(Default)]
pub struct AppBuilder {
    pub world: World,
    pub startup: Schedule,
    pub update: Schedule,
    pub render: Schedule,
    pub unlink: Schedule,
}

#[macroquad::main("Demo App")]
async fn main() {
    let mut app = AppBuilder::default();
    game::build(&mut app);

    let AppBuilder {
        mut world,
        mut startup,
        mut update,
        mut render,
        mut unlink,
    } = app;

    startup.run(&mut world);

    while !is_quit_requested() && !is_key_pressed(KeyCode::Escape) {
        update.run(&mut world);
        render.run(&mut world);
        unlink.run(&mut world);
        next_frame().await;
    }
}

#![feature(arbitrary_self_types)]

use bevy_ecs::{schedule::Schedule, world::World};
use macroquad::{
    color::RED,
    input::{is_key_pressed, is_quit_requested, KeyCode},
    text::draw_text,
    window::next_frame,
};
use util::arena::{make_unlinker_system, RandomComponent};

pub mod game;
pub mod util;

#[derive(Default)]
pub struct AppBuilder {
    pub world: World,
    pub startup: Schedule,
    pub update: Schedule,
    pub render: Schedule,
    pub unlinker: Schedule,
}

impl AppBuilder {
    pub fn add_unlinker<T: RandomComponent>(&mut self) {
        self.unlinker.add_systems(make_unlinker_system::<T>());
    }
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
        mut unlinker,
    } = app;

    startup.run(&mut world);
    startup.apply_deferred(&mut world);

    while !is_quit_requested() && !is_key_pressed(KeyCode::Escape) {
        update.run(&mut world);
        update.apply_deferred(&mut world);

        render.run(&mut world);
        render.apply_deferred(&mut world);

        draw_text(
            &format!("Entities: {}", world.entities().len()),
            10.,
            10.,
            14.,
            RED,
        );

        unlinker.run(&mut world);
        unlinker.apply_deferred(&mut world);

        next_frame().await;
    }
}

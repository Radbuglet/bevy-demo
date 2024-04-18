#![feature(arbitrary_self_types)]

use bevy_ecs::{
    schedule::{LogLevel, Schedule, ScheduleBuildSettings},
    world::World,
};
use macroquad::{
    color::RED,
    input::{is_key_pressed, is_quit_requested, KeyCode},
    text::draw_text,
    window::next_frame,
};
use util::arena::{make_unlinker_system, RandomArena, RandomComponent};

pub mod game;
pub mod util;

pub struct AppBuilder {
    pub world: World,
    pub startup: Schedule,
    pub update: Schedule,
    pub render: Schedule,
    pub disposer: Schedule,
    pub unlinker: Schedule,
}

fn make_unambiguous_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_build_settings(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Error,
        ..Default::default()
    });
    schedule
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            world: World::default(),
            startup: make_unambiguous_schedule(),
            update: make_unambiguous_schedule(),
            render: make_unambiguous_schedule(),
            disposer: make_unambiguous_schedule(),
            unlinker: make_unambiguous_schedule(),
        }
    }
}

impl AppBuilder {
    pub fn add_unlinker<T: RandomComponent>(&mut self) {
        self.unlinker.add_systems(make_unlinker_system::<T>());
        self.world.init_resource::<RandomArena<T>>();
    }
}

#[macroquad::main("Demo App")]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    color_backtrace::install();

    let mut app = AppBuilder::default();
    game::build(&mut app);

    let AppBuilder {
        mut world,
        mut startup,
        mut update,
        mut render,
        mut disposer,
        mut unlinker,
    } = app;

    startup.run(&mut world);

    while !is_quit_requested() && !is_key_pressed(KeyCode::Escape) {
        update.run(&mut world);
        render.run(&mut world);
        disposer.run(&mut world);

        draw_text(
            &format!("Entities: {}", world.entities().len()),
            10.,
            10.,
            14.,
            RED,
        );

        unlinker.run(&mut world);

        next_frame().await;
    }
}

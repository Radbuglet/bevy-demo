#![feature(arbitrary_self_types)]
#![allow(clippy::type_complexity)]

use bevy_app::App;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel};
use macroquad::{
    color::RED,
    input::{is_key_pressed, is_quit_requested, KeyCode},
    text::draw_text,
    window::next_frame,
};

#[derive(ScheduleLabel, Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Render;

pub mod game;
pub mod util;

#[macroquad::main("Bevy Demo")]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    color_backtrace::install();

    let mut app = App::new();
    app.configure_schedules(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Error,
        hierarchy_detection: LogLevel::Warn,
        ..Default::default()
    });
    app.add_plugins(game::plugin);

    while !is_quit_requested() && !is_key_pressed(KeyCode::Escape) {
        app.update();
        app.world.run_schedule(Render);
        draw_text(
            &format!("Entities: {}", app.world.entities().total_count()),
            15.,
            15.,
            24.,
            RED,
        );
        next_frame().await;
    }
}

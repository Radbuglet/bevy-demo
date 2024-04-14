#![feature(arbitrary_self_types)]

pub mod engine;
pub mod game;
pub mod util;

#[macroquad::main("Demo App")]
async fn main() {
    engine::entry::main_inner().await;
}

#![feature(arbitrary_self_types)]
#![feature(ptr_metadata)]
#![feature(unsize)]

pub mod game;
pub mod util;

#[macroquad::main("Demo App")]
async fn main() {
    game::entry::main_inner().await;
}

use macroquad::{input::is_quit_requested, window::next_frame};

use crate::util::arena::Universe;

pub async fn main_inner() {
    let mut universe = Universe::new();

    universe.queue_task::<()>(|| {
        println!("This only runs once.");
    });

    while !is_quit_requested() {
        next_frame().await;
    }
}

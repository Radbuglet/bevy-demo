use macroquad::{input::is_quit_requested, window::next_frame};

use crate::{
    game::entry::make_scene,
    util::arena::{StrongEntity, Universe},
};

use super::scene::{SceneManager, SceneUpdateHandler};

pub async fn main_inner() {
    let universe = Universe::new();

    // Create engine root
    let engine_root = universe.run::<&mut SceneManager, _>(|| {
        let root = StrongEntity::new();
        root.insert(SceneManager::default());
        root
    });

    // Setup initial scene
    {
        let scene = make_scene(&universe);
        universe.run::<&mut SceneManager, _>(|| {
            engine_root.get::<SceneManager>().set_initial(scene);
        });
    }

    // Run main loop
    while !is_quit_requested() {
        let update_handler = universe.run::<(&SceneManager, &SceneUpdateHandler), _>(|| {
            engine_root
                .get::<SceneManager>()
                .current()
                .get::<SceneUpdateHandler>()
                .deref()
                .clone()
        });

        update_handler(&universe);

        universe.run::<&mut SceneManager, _>(|| {
            engine_root.get::<SceneManager>().swap_scenes();
        });

        next_frame().await;
    }
}

use macroquad::{input::is_quit_requested, window::next_frame};

use crate::{
    component,
    util::arena::{StrongEntity, Universe},
};

use super::scene::{SceneManager, SceneUpdateHandler};

component!(u32);

pub async fn main_inner() {
    let universe = Universe::new();
    let engine_root =
        universe.run::<(&mut SceneManager, &mut SceneUpdateHandler, &mut u32), _>(|| {
            let root = StrongEntity::new();
            let sm = root.insert(SceneManager::default());

            // Setup initial scene
            let (scene, scene_ref) = StrongEntity::new().split_guard();
            scene.insert(3u32);
            scene.insert(SceneUpdateHandler::new(move |universe| {
                universe.run::<&mut u32, _>(|| {
                    *scene_ref.get::<u32>() += 1;
                    dbg!(*scene_ref.get::<u32>());
                });
            }));
            sm.deref_mut().set_initial(scene);

            root
        });

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

        universe.run::<&mut SceneManager, _>(|| engine_root.get::<SceneManager>().swap_scenes());

        next_frame().await;
    }
}

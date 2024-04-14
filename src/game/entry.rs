use autoken::BorrowsMut;
use macroquad::math::Vec2;

use crate::util::arena::{spawn_universe_task, CompBorrowsExt, Entity, Universe};

use super::{
    colliders::Collider,
    scene::{SceneManager, SceneUpdateHandler},
    spatial::Spatial,
};

pub async fn main_inner() {
    let mut universe = Universe::new();

    let root = universe.acquire::<(
        &mut Spatial,
        &mut Collider,
        &mut SceneManager,
        &mut SceneUpdateHandler,
    ), _>(|| {
        let root = Entity::new();
        root.insert(Spatial::from_pos(Vec2::ONE));
        root.insert(SceneManager::default());

        let main_scene = Entity::new();
        let main_scene_spatial = main_scene.insert(Spatial::from_pos(Vec2::ZERO));
        main_scene_spatial.set_parent(Some(root.get()));

        main_scene.insert(SceneUpdateHandler::new(move |cx| {
            cx.spawn_universe_task::<&mut Spatial>(move || {
                dbg!(main_scene_spatial.global_pos());
            });
        }));

        root.get::<SceneManager>().set_initial(main_scene);

        root
    });

    universe.spawn::<(&mut SceneManager, &mut Spatial, &SceneUpdateHandler)>(move || {
        let mut sm = root.get::<SceneManager>();
        let scene = sm.current();

        dbg!(scene.get::<Spatial>().global_pos());

        spawn_universe_task::<()>(|| {
            println!("Hi!");
        });

        spawn_universe_task::<()>(|| {
            println!("Hello!");
        });

        (scene.get::<SceneUpdateHandler>())(&mut BorrowsMut::acquire());

        sm.swap_scenes();
    });

    universe.dispatch();
}

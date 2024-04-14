use crate::{
    component,
    engine::scene::SceneUpdateHandler,
    util::{
        arena::{Entity, StrongEntity, Universe},
        deferred::{DeferQueue, Deferred},
    },
};

component!(SceneState);

pub fn make_scene(universe: &Universe) -> StrongEntity {
    let mut queue = DeferQueue::new();

    let scene = universe.run_with_queue::<(&mut SceneUpdateHandler, &mut SceneState), _>(
        &mut queue,
        || {
            let (scene, scene_ref) = StrongEntity::new().split_guard();

            Deferred::new::<&mut SceneState, _>(|my_entity: Entity| {
                my_entity.get::<SceneState>().counter += 5;
            })
            .queue_run(scene_ref);

            let scene_state = scene.insert(SceneState { counter: 0 });

            scene.insert(SceneUpdateHandler::new(move |universe| {
                let mut scene_state = scene_state;

                universe.run::<&mut SceneState, _>(|| {
                    scene_state.counter += 1;

                    dbg!(scene_state.counter);
                });
            }));

            scene
        },
    );

    queue.run(universe);
    scene
}

#[derive(Debug)]
pub struct SceneState {
    counter: u32,
}

#[derive(Debug)]
pub struct Service {}

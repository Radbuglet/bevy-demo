use std::time::Instant;

use bevy_app::{App, Startup, Update};
use bevy_ecs::{
    component::Component,
    entity::Entities,
    system::{Local, Query},
};

use crate::{
    random_component,
    util::arena::{
        spawn_entity, Obj, ObjOwner, RandomAccess, RandomAppExt, RandomComponent, RandomEntityExt,
    },
};

// === DemoComp === //

random_component!(DemoRandComp);

#[derive(Debug)]
struct DemoRandComp(u32);

#[derive(Debug, Component)]
struct DemoComp(u32);

// === Systems === //

pub fn plugin(app: &mut App) {
    app.add_random_component::<DemoRandComp>();
    app.add_systems(Startup, sys_init);
    app.add_systems(Update, (sys_update_rand, sys_update_rand2, sys_update_ecs));
}

fn sys_init(mut rand: RandomAccess<&mut DemoRandComp>, spawner: &Entities) {
    rand.provide(|| {
        let batch = spawner.reserve_entities(10_000);

        for entity in batch {
            entity.insert(DemoRandComp(0));
        }

        for _ in 0..10_000 {
            spawn_entity((DemoComp(0),));
        }
    });
}

fn sys_update_rand(
    mut rand: RandomAccess<&mut DemoRandComp>,
    mut query: Query<&ObjOwner<DemoRandComp>>,
    mut local: Local<(f64, f64)>,
) {
    rand.provide(|| {
        let start = Instant::now();
        for &ObjOwner(mut value) in query.iter_mut() {
            value.0 += 1;
        }

        local.0 += start.elapsed().as_secs_f64() * 10E9 / 10_000.;
        local.1 += 1.;

        dbg!(local.0 / local.1);
    });
}

fn sys_update_rand2(
    mut rand: RandomAccess<&mut DemoRandComp>,
    mut query: Query<&ObjOwner<DemoRandComp>>,
    mut local: Local<(f64, f64)>,
) {
    rand.provide(|| {
        let arena = DemoRandComp::arena_mut();
        let start = Instant::now();
        for &ObjOwner(value) in query.iter_mut() {
            (arena.arena[Obj::index(value)].1).0 += 1;
        }

        local.0 += start.elapsed().as_secs_f64() * 10E9 / 10_000.;
        local.1 += 1.;

        dbg!(local.0 / local.1);
    });
}

fn sys_update_ecs(mut query: Query<&mut DemoComp>, mut local: Local<(f64, f64)>) {
    let start = Instant::now();

    for mut comp in query.iter_mut() {
        comp.0 += 1;
    }

    local.0 += start.elapsed().as_secs_f64() * 10E9 / 10_000.;
    local.1 += 1.;

    dbg!(local.0 / local.1);
}

use bevy_ecs::{
    component::Component,
    entity::Entity,
    schedule::Schedule,
    system::{Res, Resource},
    world::World,
};
use util::ecs::{RandomAccess, RandomEntityExt, RandomQuery};

pub mod util;

#[derive(Debug, Resource)]
pub struct GlobalCounter(Entity);

#[derive(Debug, Component)]
pub struct Counter(pub u32);

fn main() {
    // Build world
    let mut world = World::new();
    let counter = world.spawn(Counter(0)).id();
    world.insert_resource(GlobalCounter(counter));

    // Build schedule
    let mut schedule = Schedule::default();
    schedule.add_systems(whee);

    // Run
    schedule.run(&mut world);
}

random_component!(Counter);

fn whee(query: RandomAccess<&'static mut Counter>, counter: Res<GlobalCounter>) {
    let counter = counter.0;

    query.provide(|| {
        counter.get_mut::<Counter>().0 += 1;
        dbg!(counter.get::<Counter>());
    });
}

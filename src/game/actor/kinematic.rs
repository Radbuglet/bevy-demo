use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventWriter},
    query::With,
    system::{Query, Res},
};
use cbit::cbit;
use macroquad::{
    color::{Color, BLUE},
    math::Vec2,
    shapes::draw_rectangle,
};
use rustc_hash::FxHashSet;

use crate::{
    game::{
        math::{aabb::Aabb, glam::Vec2Ext},
        tile::{
            collider::{
                Collider, InsideWorld, TrackedCollider, TrackedColliderChunk, WorldColliders,
            },
            data::{TileChunk, TileWorld, WorldCreatedChunk},
            kinematic::{AnyCollision, KinematicApi, TileColliderDescriptor},
            material::MaterialRegistry,
        },
    },
    util::arena::{RandomAccess, RandomEntityExt, SendsEvent},
};

use super::camera::ActiveCamera;

// === Systems === //

#[derive(Debug, Component)]
pub struct Pos(pub Vec2);

#[derive(Debug, Component)]
pub struct Vel(pub Vec2);

#[derive(Debug, Component, Default)]
pub struct ColliderMoves;

#[derive(Debug, Component, Default)]
pub struct ColliderListens {
    contains: FxHashSet<Entity>,
}

#[derive(Debug, Event)]
pub struct ColliderEvent {
    pub listener: Entity,
    pub other: Entity,
    pub entered: bool,
}

pub fn sys_update_moving_colliders(
    mut query: Query<(&InsideWorld, &mut Pos, &mut Vel, &mut Collider), With<ColliderMoves>>,
    mut rand: RandomAccess<(
        &mut TileWorld,
        &mut TileChunk,
        &mut KinematicApi,
        &mut TrackedColliderChunk,
        &TrackedCollider,
        &WorldColliders,
        &TileColliderDescriptor,
        &MaterialRegistry,
        SendsEvent<WorldCreatedChunk>,
    )>,
) {
    rand.provide(|| {
        for (&InsideWorld(world), mut pos, mut vel, mut collider) in query.iter_mut() {
            let mut world = world.entity().get::<KinematicApi>();

            let delta = vel.0;
            let filter = |coll| match coll {
                AnyCollision::Tile(_, _, _) => true,
                AnyCollision::Collider(_, _) => false,
            };

            let delta = world.move_by(collider.0, delta, filter);
            pos.0 += delta;
            collider.0 = Aabb::new_centered(pos.0, Vec2::splat(40.));

            let mask = world.get_clip_mask(collider.0, vel.0, filter);
            vel.0 = vel.0.mask(mask);
        }
    });
}

pub fn sys_update_listening_colliders(
    mut rand: RandomAccess<(
        &mut TileWorld,
        &mut TileChunk,
        &mut WorldColliders,
        &mut TrackedColliderChunk,
        &TrackedCollider,
        SendsEvent<WorldCreatedChunk>,
    )>,
    mut query: Query<(Entity, &InsideWorld, &Collider, &mut ColliderListens)>,
    mut events: EventWriter<ColliderEvent>,
) {
    rand.provide(|| {
        let mut removed = FxHashSet::default();

        for (listener, &InsideWorld(world), &Collider(aabb), mut listen_state) in query.iter_mut() {
            let world = world.entity().get::<WorldColliders>();

            removed.clear();
            removed.extend(listen_state.contains.drain());

            cbit! {
                for (other, _) in world.collisions(aabb) {
                    if listener == other {
                        continue;
                    }

                    listen_state.contains.insert(other);
                    if !removed.remove(&other) {
                        log::info!("Enter: {other:?} (listener: {listener:?})");
                        events.send(ColliderEvent { listener, other, entered: true });
                    }
                }
            }

            for other in removed.drain() {
                log::info!("Exit: {other:?} (listener: {listener:?})");
                events.send(ColliderEvent {
                    listener,
                    other,
                    entered: false,
                });
            }
        }
    });
}

pub fn sys_draw_debug_colliders(mut query: Query<&Collider>, camera: Res<ActiveCamera>) {
    let _guard = camera.apply();

    for &Collider(aabb) in query.iter_mut() {
        draw_rectangle(
            aabb.x(),
            aabb.y(),
            aabb.w(),
            aabb.h(),
            Color::from_vec(BLUE.to_vec().truncate().extend(0.3)),
        );
    }
}

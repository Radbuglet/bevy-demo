use std::f32::consts::TAU;

use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    event::EventReader,
    query::With,
    system::{Commands, Query, Res},
};
use macroquad::{color::BLUE, math::Vec2, rand::gen_range, shapes::draw_circle};

use crate::{
    game::{
        math::aabb::Aabb,
        tile::{
            collider::{Collider, InsideWorld},
            data::TileWorld,
            kinematic::TangibleMarker,
        },
    },
    util::arena::{despawn_entity, RandomAccess, RandomEntityExt},
};

use super::{
    camera::ActiveCamera,
    health::Health,
    kinematic::{ColliderEvent, ColliderListens, ColliderMoves, Pos, Vel},
    player::PlayerState,
};

// === Systems === //

#[derive(Bundle)]
pub struct BulletBaseBundle {
    pub pos: Pos,
    pub vel: Vel,
    pub world: InsideWorld,
    pub collider: Collider,
    pub moves: ColliderMoves,
    pub listens: ColliderListens,
    pub damage: BulletDamage,
}

#[derive(Debug, Component)]
pub struct BulletDamage {
    pub amount: f32,
    pub despawn: bool,
}

#[derive(Debug, Component)]
pub struct BulletSpawner;

pub fn sys_apply_bullet_damage(
    mut events: EventReader<ColliderEvent>,
    mut bullet_query: Query<&BulletDamage>,
    mut player_query: Query<&InsideWorld, With<PlayerState>>,
    mut rand: RandomAccess<(&TileWorld, &mut Health)>,
) {
    rand.provide(|| {
        for event in events.read() {
            if !event.entered {
                continue;
            }

            let Ok(bullet) = bullet_query.get_mut(event.listener) else {
                continue;
            };

            let Ok(&InsideWorld(world)) = player_query.get_mut(event.other) else {
                continue;
            };

            world.entity().get::<Health>().change_health(-bullet.amount);

            if bullet.despawn {
                despawn_entity(event.listener);
            }
        }
    });
}

pub fn sys_tick_bullet_spawner(
    mut query: Query<(&InsideWorld, &Pos), With<BulletSpawner>>,
    mut rand: RandomAccess<&mut TangibleMarker>,
    mut commands: Commands,
) {
    rand.provide(|| {
        for (&InsideWorld(world), &Pos(pos)) in query.iter_mut() {
            let entity = commands
                .spawn(BulletBaseBundle {
                    pos: Pos(pos),
                    vel: Vel(Vec2::from_angle(gen_range(0., TAU)) * 10.),
                    world: InsideWorld(world),
                    collider: Collider(Aabb::ZERO),
                    moves: ColliderMoves,
                    listens: ColliderListens::default(),
                    damage: BulletDamage {
                        despawn: true,
                        amount: 2.,
                    },
                })
                .id();

            entity.insert(TangibleMarker);
        }
    });
}

pub fn sys_render_bullets(mut query: Query<&Pos, With<BulletDamage>>, camera: Res<ActiveCamera>) {
    let _guard = camera.apply();

    for &Pos(pos) in query.iter_mut() {
        draw_circle(pos.x, pos.y, 20., BLUE);
    }
}

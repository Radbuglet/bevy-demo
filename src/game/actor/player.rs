use std::collections::VecDeque;

use bevy_ecs::{
    component::Component,
    event::EventReader,
    query::With,
    system::{Query, Res, ResMut},
};
use macroquad::{
    color::{Color, DARKPURPLE, GRAY, GREEN, RED, WHITE, YELLOW},
    input::{is_key_down, is_mouse_button_down, mouse_position, KeyCode, MouseButton},
    math::{Affine2, IVec2, Vec2},
    miniquad::window::screen_size,
    shapes::{draw_circle, draw_rectangle},
};

use crate::{
    game::{
        math::aabb::Aabb,
        tile::{
            collider::{
                Collider, InsideWorld, TrackedCollider, TrackedColliderChunk, WorldColliders,
            },
            data::{TileChunk, TileLayerConfig, TileWorld, WorldCreatedChunk},
            kinematic::{
                filter_tangible_actors, KinematicApi, TangibleMarker, TileColliderDescriptor,
            },
            material::{BaseMaterialDescriptor, MaterialId, MaterialRegistry},
            render::{RenderableWorld, SolidTileMaterial},
        },
    },
    random_component,
    util::arena::{spawn_entity, ObjOwner, RandomAccess, RandomEntityExt, SendsEvent},
};

use super::{
    camera::{ActiveCamera, VirtualCamera, VirtualCameraConstraints},
    kinematic::{ColliderEvent, ColliderListens, ColliderMoves, Pos, Vel},
};

random_component!(Health);

// === Health === //

#[derive(Debug)]
pub struct Health {
    health: f32,
    max: f32,
}

impl Health {
    pub fn new(health: f32, max: f32) -> Self {
        let max = max.max(0.);
        let health = health.clamp(0., max);

        Self { health, max }
    }

    pub fn new_full(max: f32) -> Self {
        Self::new(max, max)
    }

    pub fn health(&self) -> f32 {
        self.health
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    pub fn set_health(&mut self, health: f32) {
        self.health = health.clamp(0., self.max);
    }

    pub fn set_max(&mut self, max: f32) {
        self.max = max.max(0.);
        self.health = self.health.min(self.max);
    }

    pub fn change_health(&mut self, amount: f32) {
        self.set_health(self.health() + amount);
    }

    pub fn change_max(&mut self, by: f32) {
        self.set_max(self.max() + by);
    }

    pub fn reheal(&mut self) {
        self.health = self.max;
    }

    pub fn is_alive(&self) -> bool {
        self.health != 0.
    }

    pub fn percentage(&self) -> f32 {
        self.health / self.max
    }
}

// === Systems === //

#[derive(Component)]
pub struct Player {
    trail: VecDeque<Vec2>,
}

#[derive(Component)]
pub struct HealthAnimation(f32);

pub fn sys_create_local_player(
    mut rand: RandomAccess<(
        &mut BaseMaterialDescriptor,
        &mut Health,
        &mut KinematicApi,
        &mut MaterialRegistry,
        &mut SolidTileMaterial,
        &mut TangibleMarker,
        &mut TileChunk,
        &mut TileColliderDescriptor,
        &mut TileWorld,
        &mut VirtualCamera,
        &mut WorldColliders,
        SendsEvent<WorldCreatedChunk>,
    )>,
    mut camera: ResMut<ActiveCamera>,
) {
    rand.provide(|| {
        // Spawn world
        let world = spawn_entity((HealthAnimation(1.), RenderableWorld::default()));

        // Setup camera
        camera.camera = Some(world.insert(VirtualCamera::new(
            Affine2::IDENTITY,
            Aabb::new_centered(Vec2::ZERO, Vec2::splat(1000.)),
            VirtualCameraConstraints::default().keep_visible_area(Vec2::new(1000., 1000.)),
        )));

        // Setup material registry
        let mut registry = world.insert(MaterialRegistry::default());
        registry.register("game:air", spawn_entity(()));
        let grass = registry.register("game:grass", {
            let descriptor = spawn_entity(());
            descriptor.insert(SolidTileMaterial { color: GREEN });
            descriptor.insert(TileColliderDescriptor::new([Aabb::ZERO_TO_ONE]));
            descriptor
        });
        let stone = registry.register("game:stone", {
            let descriptor = spawn_entity(());
            descriptor.insert(SolidTileMaterial { color: GRAY });
            descriptor.insert(TileColliderDescriptor::new([Aabb::ZERO_TO_ONE]));
            descriptor
        });

        // Setup world
        let world_data = world.insert(TileWorld::new(TileLayerConfig {
            offset: Vec2::ZERO,
            size: 50.,
        }));
        let world_colliders = world.insert(WorldColliders::new(world_data));

        for x in 0..500 {
            let v = (x as f32 / 10.).sin();
            world_data.set_tile(IVec2::new(x, (v * 10.) as i32), grass);
            world_data.set_tile(IVec2::new(x, (v * 10.) as i32 - 20), stone);
        }

        world.insert(KinematicApi::new(world_data, registry, world_colliders));

        // Setup health
        world.insert(Health::new_full(50.));

        // Spawn player
        let player = spawn_entity((
            Pos(Vec2::ZERO),
            Vel(Vec2::ONE),
            InsideWorld(world_data),
            Collider(Aabb::ZERO_TO_ONE),
            ColliderMoves,
            Player {
                trail: VecDeque::new(),
            },
        ));
        player.insert(TangibleMarker);

        // Spawn listener
        spawn_entity((
            InsideWorld(world_data),
            Collider(Aabb::new(100., 100., 500., 500.)),
            ColliderListens::default(),
        ));
    });
}

pub fn sys_handle_controls(
    mut rand: RandomAccess<(
        &MaterialRegistry,
        &mut KinematicApi,
        &mut TileChunk,
        &mut TileWorld,
        &mut VirtualCamera,
        &mut WorldColliders,
        &TangibleMarker,
        &TileColliderDescriptor,
        &TrackedCollider,
        &TrackedColliderChunk,
        SendsEvent<WorldCreatedChunk>,
    )>,
    mut query: Query<(&InsideWorld, &Pos, &mut Vel, &mut Player)>,
) {
    rand.provide(|| {
        let mut heading = Vec2::ZERO;
        if is_key_down(KeyCode::A) {
            heading += Vec2::NEG_X;
        }
        if is_key_down(KeyCode::D) {
            heading += Vec2::X;
        }
        if is_key_down(KeyCode::W) {
            heading += Vec2::NEG_Y;
        }
        if is_key_down(KeyCode::S) {
            heading += Vec2::Y;
        }

        heading = heading.normalize_or_zero();

        for (&InsideWorld(world), pos, mut vel, mut player) in query.iter_mut() {
            let camera = world.entity().get::<VirtualCamera>();
            let registry = world.entity().get::<MaterialRegistry>();
            let mut kinematics = world.entity().get::<KinematicApi>();

            vel.0 += heading;
            vel.0 *= 0.98;

            player.trail.push_front(pos.0);
            if player.trail.len() > 100 {
                player.trail.pop_back();
            }

            let hover = Vec2::from(mouse_position());
            let hover = camera.project(hover);
            let hover = world.config().actor_to_tile(hover);

            if is_mouse_button_down(MouseButton::Left) {
                world.set_tile(hover, MaterialId::AIR);
            } else if is_mouse_button_down(MouseButton::Right) {
                let place_aabb = world
                    .config()
                    .tile_to_actor_rect(hover)
                    .shrink(Vec2::splat(0.01));

                if kinematics.has_colliders_in(place_aabb, filter_tangible_actors) {
                    continue;
                }

                if world.tile(hover) != MaterialId::AIR {
                    continue;
                }

                world.set_tile(hover, registry.lookup_by_name("game:stone").unwrap());
            }
        }
    });
}

pub fn sys_handle_damage(
    mut rand: RandomAccess<(&TileWorld, &mut Health)>,
    mut query: Query<&InsideWorld, With<Player>>,
    mut events: EventReader<ColliderEvent>,
) {
    rand.provide(|| {
        for event in events.read() {
            if !event.entered {
                continue;
            }

            let Ok(&InsideWorld(world)) = query.get_mut(event.other) else {
                continue;
            };

            world.entity().get::<Health>().change_health(-2.);
        }
    });
}

pub fn sys_focus_camera_on_player(
    mut query: Query<(&InsideWorld, &Pos), With<Player>>,
    mut rand: RandomAccess<(&mut TileWorld, &mut VirtualCamera)>,
) {
    rand.provide(|| {
        let Some((&InsideWorld(world), pos)) = query.iter_mut().next() else {
            return;
        };

        world
            .entity()
            .get::<VirtualCamera>()
            .set_transform(Affine2::from_translation(pos.0));
    });
}

pub fn sys_render_players(
    mut rand: RandomAccess<(&TileWorld, &mut VirtualCamera)>,
    mut query: Query<(&InsideWorld, &Pos, &Player)>,
    camera: Res<ActiveCamera>,
) {
    let _guard = camera.apply();

    rand.provide(|| {
        for (&InsideWorld(world), pos, player) in query.iter_mut() {
            let config = world.config();

            // Draw placement indicator
            {
                let pos = Vec2::from(mouse_position());
                let pos = camera.camera.unwrap().project(pos);
                let pos = config.tile_to_actor_rect(config.actor_to_tile(pos));

                draw_rectangle(pos.x(), pos.y(), pos.w(), pos.h(), RED);
            }

            // Draw player
            for (i, &trail) in player.trail.iter().rev().enumerate() {
                draw_circle(
                    trail.x,
                    trail.y,
                    20.,
                    Color::from_vec(
                        DARKPURPLE
                            .to_vec()
                            .lerp(RED.to_vec(), i as f32 / player.trail.len() as f32),
                    ),
                );
            }

            draw_circle(pos.0.x, pos.0.y, 20., RED);
        }
    });
}

pub fn sys_render_health_bar(
    mut rand: RandomAccess<&Health>,
    mut query: Query<(&ObjOwner<Health>, &mut HealthAnimation), With<ObjOwner<TileWorld>>>,
) {
    let screen_size = Vec2::from(screen_size());

    rand.provide(|| {
        for (&ObjOwner(hp), mut hp_anim) in query.iter_mut() {
            let aabb = Aabb::new_centered(
                Vec2::new(screen_size.x / 2., screen_size.y - 20.),
                Vec2::new(screen_size.x * 0.8, 10.),
            );

            let aabb2 = aabb.grow(Vec2::splat(5.));
            draw_rectangle(aabb2.x(), aabb2.y(), aabb2.w(), aabb2.h(), WHITE);

            let hp_active = hp.percentage();
            hp_anim.0 = (hp_anim.0 + hp_active) / 2.;

            draw_rectangle(aabb.x(), aabb.y(), aabb.w(), aabb.h(), RED);
            draw_rectangle(
                aabb.x(),
                aabb.y(),
                aabb.w() * hp.percentage(),
                aabb.h(),
                GREEN,
            );

            if hp_anim.0 > hp_active {
                let aabb3 = Aabb::new_poly(&[
                    aabb.point_at(Vec2::new(hp_active, 0.)),
                    aabb.point_at(Vec2::new(hp_anim.0, 1.)),
                ]);
                draw_rectangle(aabb3.x(), aabb3.y(), aabb3.w(), aabb3.h(), YELLOW);
            }
        }
    });
}

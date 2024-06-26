use std::collections::VecDeque;

use bevy_ecs::{
    component::Component,
    event::EventReader,
    query::With,
    system::{Query, Res, ResMut},
};
use cbit::cbit;
use macroquad::{
    color::{Color, DARKPURPLE, GRAY, GREEN, RED, WHITE, YELLOW},
    input::{is_key_down, is_mouse_button_down, mouse_position, KeyCode, MouseButton},
    math::{Affine2, IVec2, Vec2},
    miniquad::window::screen_size,
    shapes::draw_circle,
};

use crate::{
    game::{
        math::{
            aabb::Aabb,
            draw::{draw_rectangle_aabb, stroke_rectangle_aabb},
        },
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
    util::arena::{spawn_entity, ObjOwner, RandomAccess, RandomEntityExt, SendsEvent},
};

use super::{
    camera::{ActiveCamera, VirtualCamera, VirtualCameraConstraints},
    health::Health,
    kinematic::{ColliderEvent, ColliderListens, ColliderMoves, Pos, Vel},
    projectile::BulletSpawner,
};

// === Systems === //

#[derive(Component, Default)]
pub struct WorldState {
    focused_tile: Vec2,
}

#[derive(Component, Default)]
pub struct PlayerState {
    trail: VecDeque<Vec2>,
    last_tile: Option<Vec2>,
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
        let world = spawn_entity((
            HealthAnimation(1.),
            RenderableWorld::default(),
            WorldState::default(),
        ));

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
            Pos(Vec2::new(0., -50.)),
            Vel(Vec2::ONE),
            InsideWorld(world_data),
            Collider(Aabb::ZERO),
            ColliderMoves,
            PlayerState::default(),
        ));
        player.insert(TangibleMarker);

        spawn_entity((
            Pos(Vec2::new(-500., -200.)),
            InsideWorld(world_data),
            BulletSpawner,
        ));

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
    mut query: Query<(&InsideWorld, &Pos, &mut Vel, &mut PlayerState)>,
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
            let config = world.config();
            let camera = world.entity().get::<VirtualCamera>();
            let registry = world.entity().get::<MaterialRegistry>();
            let mut kinematics = world.entity().get::<KinematicApi>();

            // Update heading vector
            vel.0 += heading;
            vel.0 *= 0.98;

            // Update trail
            player.trail.push_front(pos.0);
            if player.trail.len() > 100 {
                player.trail.pop_back();
            }

            // Determine the tile over which the player's cursor is hovering.
            let dest = Vec2::from(mouse_position());
            let dest = camera.project(dest);

            let src = player.last_tile.unwrap_or(dest);
            player.last_tile = Some(dest);

            if is_mouse_button_down(MouseButton::Left) {
                cbit! {
                    for pos in config.step_ray_tiles(src, dest) {
                        world.set_tile(pos, MaterialId::AIR);
                    }
                }
            } else if is_mouse_button_down(MouseButton::Right) {
                cbit! {
                    for pos in config.step_ray_tiles(src, dest) {
                        let place_aabb = config
                            .tile_to_actor_rect(pos)
                            .shrink(Vec2::splat(0.01));

                        if kinematics.has_colliders_in(place_aabb, filter_tangible_actors) {
                            continue;
                        }

                        if world.tile(pos) != MaterialId::AIR {
                            continue;
                        }

                        world.set_tile(pos, registry.lookup_by_name("game:stone").unwrap());
                    }
                }
            } else {
                player.last_tile = None;
            }
        }
    });
}

pub fn sys_handle_damage(
    mut rand: RandomAccess<(&TileWorld, &mut Health)>,
    mut query: Query<&InsideWorld, With<PlayerState>>,
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
    mut query: Query<(&InsideWorld, &Pos), With<PlayerState>>,
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
    mut query: Query<(&Pos, &PlayerState)>,
    camera: Res<ActiveCamera>,
) {
    let _guard = camera.apply();

    rand.provide(|| {
        for (pos, player) in query.iter_mut() {
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

pub fn sys_render_selection_indicator(
    mut rand: RandomAccess<(&TileWorld, &mut VirtualCamera)>,
    mut query: Query<(&ObjOwner<TileWorld>, &mut WorldState)>,
    camera: Res<ActiveCamera>,
) {
    let _guard = camera.apply();

    rand.provide(|| {
        for (&ObjOwner(world), mut world_state) in query.iter_mut() {
            let config = world.config();

            let pos = Vec2::from(mouse_position());
            let pos = camera.camera.unwrap().project(pos);
            let pos = config.actor_to_tile(pos).as_vec2();

            world_state.focused_tile = (world_state.focused_tile + pos * 5.) / (1. + 5.);

            let aabb = config.floating_tile_to_actor_rect(world_state.focused_tile);

            stroke_rectangle_aabb(aabb, 2., RED);
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

            draw_rectangle_aabb(aabb.grow(Vec2::splat(5.)), WHITE);

            let hp_active = hp.percentage();
            hp_anim.0 = (hp_anim.0 + hp_active) / 2.;

            draw_rectangle_aabb(aabb, RED);
            draw_rectangle_aabb(aabb.with_width(aabb.w() * hp.percentage()), GREEN);

            if hp_anim.0 > hp_active {
                draw_rectangle_aabb(
                    Aabb::new_poly(&[
                        aabb.point_at(Vec2::new(hp_active, 0.)),
                        aabb.point_at(Vec2::new(hp_anim.0, 1.)),
                    ]),
                    YELLOW,
                );
            }
        }
    });
}

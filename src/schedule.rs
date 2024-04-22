use bevy_app::{App, Startup, Update};

use crate::{
    game::{
        actor::{
            camera::{sys_update_camera, ActiveCamera, VirtualCamera},
            health::Health,
            kinematic::{
                sys_draw_debug_colliders, sys_update_listening_colliders,
                sys_update_moving_colliders, ColliderEvent,
            },
            player::{
                sys_create_local_player, sys_focus_camera_on_player, sys_handle_controls,
                sys_handle_damage, sys_render_health_bar, sys_render_players,
                sys_render_selection_indicator,
            },
        },
        tile::{
            collider::{
                sys_add_collider_to_new_chunk, sys_add_tracked_collider_to_collider,
                sys_move_tracked_colliders, sys_remove_tracked_collider, TrackedCollider,
                TrackedColliderChunk, WorldColliders,
            },
            data::{sys_unregister_chunk_from_world, TileChunk, TileWorld, WorldCreatedChunk},
            kinematic::{KinematicApi, TangibleMarker, TileColliderDescriptor},
            material::{BaseMaterialDescriptor, MaterialRegistry},
            render::{sys_render_chunks, SolidTileMaterial},
        },
    },
    util::{arena::RandomAppExt, schedule::chain_ambiguous},
    Render,
};

pub fn plugin(app: &mut App) {
    // Components
    app.add_random_component::<BaseMaterialDescriptor>();
    app.add_random_component::<Health>();
    app.add_random_component::<KinematicApi>();
    app.add_random_component::<MaterialRegistry>();
    app.add_random_component::<SolidTileMaterial>();
    app.add_random_component::<TangibleMarker>();
    app.add_random_component::<TileChunk>();
    app.add_random_component::<TileColliderDescriptor>();
    app.add_random_component::<TileWorld>();
    app.add_random_component::<TrackedCollider>();
    app.add_random_component::<TrackedColliderChunk>();
    app.add_random_component::<VirtualCamera>();
    app.add_random_component::<WorldColliders>();

    // Resources
    app.init_resource::<ActiveCamera>();

    // Events
    app.add_event::<ColliderEvent>();
    app.add_event::<WorldCreatedChunk>();

    // Systems
    app.add_systems(Startup, chain_ambiguous(sys_create_local_player));
    app.add_systems(
        Update,
        chain_ambiguous((
            // Handle input
            sys_handle_controls,
            // Update colliders
            sys_update_moving_colliders,
            sys_update_listening_colliders,
            sys_handle_damage,
            // Update players
            sys_focus_camera_on_player,
            // Update colliders
            sys_add_collider_to_new_chunk,
            sys_add_tracked_collider_to_collider,
            sys_move_tracked_colliders,
            sys_remove_tracked_collider,
            sys_unregister_chunk_from_world,
        )),
    );
    app.add_systems(
        Render,
        chain_ambiguous((
            // Setup
            sys_update_camera,
            // Actors
            sys_render_players,
            sys_render_chunks,
            // Debug
            sys_draw_debug_colliders,
            // UI
            sys_render_selection_indicator,
            sys_render_health_bar,
        )),
    );
}

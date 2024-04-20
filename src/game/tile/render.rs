use bevy_app::App;
use bevy_ecs::{
    component::Component,
    system::{Query, Res},
};
use macroquad::{color::Color, shapes::draw_rectangle};

use crate::{
    game::actor::camera::{ActiveCamera, VirtualCamera},
    random_component,
    util::arena::{ObjOwner, RandomAccess, RandomAppExt},
    Render,
};

use super::{
    data::{TileChunk, TileWorld},
    material::{MaterialCache, MaterialId, MaterialRegistry},
};

// === RenderableWorld === //

random_component!(SolidTileMaterial);

#[derive(Debug, Default, Component)]
pub struct RenderableWorld {
    cache: MaterialCache<SolidTileMaterial>,
}

#[derive(Debug)]
pub struct SolidTileMaterial {
    pub color: Color,
}

// === Systems === //

pub fn plugin(app: &mut App) {
    app.add_random_component::<SolidTileMaterial>();
    app.add_systems(Render, (sys_render_chunks,));
}

fn sys_render_chunks(
    mut query: Query<(
        &ObjOwner<TileWorld>,
        &ObjOwner<MaterialRegistry>,
        &mut RenderableWorld,
    )>,
    mut rand: RandomAccess<(
        &TileWorld,
        &TileChunk,
        &MaterialRegistry,
        &SolidTileMaterial,
        &VirtualCamera,
    )>,
    camera: Res<ActiveCamera>,
) {
    let _guard = camera.apply();

    rand.provide(|| {
        let camera = camera.camera.unwrap();

        for (&ObjOwner(world), &ObjOwner(registry), mut cache) in query.iter_mut() {
            let config = world.config();
            let registry = &*registry;
            let cache = &mut cache.cache;

            for tile in config
                .actor_aabb_to_tile(camera.visible_aabb())
                .inclusive()
                .iter()
            {
                let material = world.tile(tile);

                if material == MaterialId::AIR {
                    continue;
                }

                let Some(material) = cache.get(registry, material) else {
                    continue;
                };

                let rect = config.tile_to_actor_rect(tile);
                draw_rectangle(rect.x(), rect.y(), rect.w(), rect.h(), material.color);
            }
        }
    });
}

use bevy_ecs::{
    component::Component,
    system::{Query, Res},
};
use macroquad::color::Color;

use crate::{
    game::{
        actor::camera::{ActiveCamera, VirtualCamera},
        math::draw::draw_rectangle_aabb,
    },
    random_component,
    util::arena::{ObjOwner, RandomAccess},
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

pub fn sys_render_chunks(
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

                draw_rectangle_aabb(config.tile_to_actor_rect(tile), material.color);
            }
        }
    });
}

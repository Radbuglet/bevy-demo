use bevy_ecs::{component::Component, system::Query};
use macroquad::{color::Color, math::IVec2, shapes::draw_rectangle};

use crate::{
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

pub fn build(app: &mut crate::AppBuilder) {
    app.add_unlinker::<SolidTileMaterial>();

    app.render.add_systems(sys_render_chunks);
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
    )>,
) {
    rand.provide(|| {
        for (&ObjOwner(world), &ObjOwner(registry), mut cache) in query.iter_mut() {
            let config = world.config();
            let registry = &*registry;
            let cache = &mut cache.cache;

            for x in 0..100 {
                for y in 0..100 {
                    let material = world.tile(IVec2::new(x, y));

                    if material == MaterialId::AIR {
                        continue;
                    }

                    let Some(material) = cache.get(registry, material) else {
                        continue;
                    };

                    let rect = config.tile_to_actor_rect(IVec2::new(x, y));
                    draw_rectangle(rect.x(), rect.y(), rect.w(), rect.h(), material.color);
                }
            }
        }
    });
}

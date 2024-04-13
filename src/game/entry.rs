use macroquad::math::Vec2;

use crate::util::arena::{Obj, World};

use super::spatial::Spatial;

pub async fn main_inner() {
    let world = World::new();

    world.acquire::<(&'static mut Spatial,), _>(|| {
        let root = Obj::new(Spatial::from_pos(Vec2::ONE));
        let child = Obj::new(Spatial::from_pos(Vec2::ZERO));

        child.set_parent(Some(root));
        dbg!(child.global_pos());
        root.set_local_pos(Vec2::NEG_ONE);
        dbg!(child.global_pos());
        child.set_parent(None);
        child.set_local_pos(Vec2::X);
        dbg!(child.global_pos());
    });
}

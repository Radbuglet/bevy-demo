use macroquad::math::Rect;

use crate::{component, util::arena::Obj};

#[derive(Debug, Default)]
pub struct ColliderManager {
    colliders: Vec<Obj<Collider>>,
}

component!(ColliderManager);

impl ColliderManager {
    pub fn register(&mut self, mut collider: Obj<Collider>) {
        collider.index = self.colliders.len();
        self.colliders.push(collider);
    }

    pub fn unregister(&mut self, collider: Obj<Collider>) {
        let index = collider.index;

        self.colliders.swap_remove(index);

        if let Some(moved) = self.colliders.get(index) {
            moved.deref_mut().index = index;
        }
    }

    pub fn intersections(&self, rect: Rect) -> impl Iterator<Item = Obj<Collider>> + '_ {
        self.colliders
            .iter()
            .copied()
            .filter(move |other| other.aabb.intersect(rect).is_some())
    }
}

#[derive(Debug, Default)]
pub struct Collider {
    index: usize,
    aabb: Rect,
}

component!(Collider);

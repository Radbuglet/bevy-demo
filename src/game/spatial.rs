use macroquad::math::{Affine2, Vec2};

use crate::{component, util::arena::Obj};

#[derive(Debug)]
pub struct Spatial {
    parent: Option<Obj<Spatial>>,
    children: Vec<Obj<Spatial>>,
    index_in_parent: usize,
    local_transform: Affine2,
    global_transform: Option<Affine2>,
}

component!(Spatial);

impl Spatial {
    pub fn from_xform(local_transform: Affine2) -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            index_in_parent: 0,
            local_transform,
            global_transform: None,
        }
    }

    pub fn from_pos(pos: Vec2) -> Self {
        Self::from_xform(Affine2::from_translation(pos))
    }

    pub fn parent(&self) -> Option<Obj<Spatial>> {
        self.parent
    }

    pub fn set_parent(mut self: Obj<Spatial>, new_parent: Option<Obj<Spatial>>) {
        if let Some(mut old_parent) = self.parent {
            let index = self.index_in_parent;
            old_parent.children.swap_remove(index);

            if let Some(mut moved) = old_parent.children.get(index).copied() {
                moved.index_in_parent = index;
            }
        }

        self.parent = new_parent;

        if let Some(mut new_parent) = new_parent {
            self.index_in_parent = new_parent.children.len();
            new_parent.children.push(self);
        }

        self.mark_dirty();
    }

    pub fn set_local_transform(mut self: Obj<Spatial>, transform: Affine2) {
        self.local_transform = transform;
        self.mark_dirty();
    }

    pub fn set_local_pos(mut self: Obj<Spatial>, pos: Vec2) {
        self.local_transform.translation = pos;
        self.mark_dirty();
    }

    pub fn local_transform(&self) -> Affine2 {
        self.local_transform
    }

    pub fn local_pos(&self) -> Vec2 {
        self.local_transform.translation
    }

    pub fn global_transform(mut self: Obj<Spatial>) -> Affine2 {
        if let Some(cached) = self.global_transform {
            return cached;
        }

        let parent_transform = self
            .parent
            .map_or(Affine2::IDENTITY, |parent| parent.global_transform());

        let my_transform = parent_transform * self.local_transform;
        self.global_transform = Some(my_transform);
        my_transform
    }

    pub fn global_pos(self: Obj<Spatial>) -> Vec2 {
        self.global_transform().translation
    }

    pub fn mark_dirty(mut self: Obj<Spatial>) {
        if self.global_transform.is_none() {
            return;
        }
        self.global_transform = None;

        for i in 0..self.children.len() {
            self.children[i].mark_dirty();
        }
    }
}

use autoken::BorrowsMut;

use crate::{
    component, delegate,
    util::arena::{CompTokensOf, Entity},
};

#[derive(Debug, Default)]
pub struct SceneManager {
    current: Option<Entity>,
    next: Option<Entity>,
}

component!(SceneManager);

impl SceneManager {
    pub fn set_initial(&mut self, scene: Entity) {
        assert!(self.current.is_none());
        self.current = Some(scene);
    }

    pub fn set_next(&mut self, scene: Entity) {
        self.next = Some(scene);
    }

    pub fn swap_scenes(&mut self) {
        if let Some(next) = self.next.take() {
            self.current = Some(next);
        }
    }

    pub fn current(&self) -> Entity {
        self.current.expect("no initial scene set")
    }
}

delegate! {
    pub fn SceneUpdateHandler(cx: &mut BorrowsMut<CompTokensOf<()>>)
}

component!(SceneUpdateHandler);

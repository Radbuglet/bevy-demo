use crate::{
    component, delegate,
    util::arena::{Entity, StrongEntity, Universe},
};

component!(SceneManager, SceneUpdateHandler);

#[derive(Debug, Default)]
pub struct SceneManager {
    current: Option<StrongEntity>,
    next: Option<StrongEntity>,
}

impl SceneManager {
    pub fn current(&self) -> Entity {
        self.current
            .as_ref()
            .map(StrongEntity::entity)
            .expect("no initial scene set")
    }

    pub fn set_initial(&mut self, scene: StrongEntity) {
        assert!(self.current.is_none());
        self.current = Some(scene);
    }

    pub fn set_next(&mut self, next: StrongEntity) {
        self.next = Some(next);
    }

    pub fn swap_scenes(&mut self) {
        if let Some(next) = self.next.take() {
            self.current = Some(next);
        }
    }
}

delegate! {
    pub fn SceneUpdateHandler(universe: &Universe)
}

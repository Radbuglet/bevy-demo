use crate::random_component;

random_component!(Health);

// === Health === //

#[derive(Debug)]
pub struct Health {
    health: f32,
    max: f32,
}

impl Health {
    pub fn new(health: f32, max: f32) -> Self {
        let max = max.max(0.);
        let health = health.clamp(0., max);

        Self { health, max }
    }

    pub fn new_full(max: f32) -> Self {
        Self::new(max, max)
    }

    pub fn health(&self) -> f32 {
        self.health
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    pub fn set_health(&mut self, health: f32) {
        self.health = health.clamp(0., self.max);
    }

    pub fn set_max(&mut self, max: f32) {
        self.max = max.max(0.);
        self.health = self.health.min(self.max);
    }

    pub fn change_health(&mut self, amount: f32) {
        self.set_health(self.health() + amount);
    }

    pub fn change_max(&mut self, by: f32) {
        self.set_max(self.max() + by);
    }

    pub fn reheal(&mut self) {
        self.health = self.max;
    }

    pub fn is_alive(&self) -> bool {
        self.health != 0.
    }

    pub fn percentage(&self) -> f32 {
        self.health / self.max
    }
}

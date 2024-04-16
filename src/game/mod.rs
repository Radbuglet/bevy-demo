pub mod actor;
pub mod math;
pub mod tile;

pub fn build(app: &mut crate::AppBuilder) {
    actor::build(app);
    tile::build(app);
}

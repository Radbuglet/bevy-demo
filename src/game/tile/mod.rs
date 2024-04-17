pub mod collider;
pub mod data;
pub mod material;
pub mod render;

pub fn build(app: &mut crate::AppBuilder) {
    collider::build(app);
    data::build(app);
    material::build(app);
    render::build(app);
}

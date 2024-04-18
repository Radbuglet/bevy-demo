use bevy_app::App;

pub mod collider;
pub mod data;
pub mod material;
pub mod render;

pub fn plugin(app: &mut App) {
    collider::plugin(app);
    data::plugin(app);
    material::plugin(app);
    render::plugin(app);
}

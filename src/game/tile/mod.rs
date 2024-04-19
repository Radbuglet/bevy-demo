use bevy_app::App;

pub mod collider;
pub mod data;
pub mod kinematic;
pub mod material;
pub mod render;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        collider::plugin,
        data::plugin,
        kinematic::plugin,
        material::plugin,
        render::plugin,
    ));
}

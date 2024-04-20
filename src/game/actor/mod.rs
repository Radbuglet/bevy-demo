use bevy_app::App;

pub mod bench;
pub mod camera;
pub mod kinematic;
pub mod player;

pub fn plugin(app: &mut App) {
    app.add_plugins((camera::plugin, kinematic::plugin, player::plugin));
}

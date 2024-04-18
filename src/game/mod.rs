use bevy_app::App;

pub mod actor;
pub mod math;
pub mod tile;

pub fn plugin(app: &mut App) {
    app.add_plugins((actor::plugin, tile::plugin));
}

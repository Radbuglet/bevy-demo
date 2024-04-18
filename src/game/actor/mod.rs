use bevy_app::App;

pub mod player;

pub fn plugin(app: &mut App) {
    app.add_plugins((player::plugin,));
}

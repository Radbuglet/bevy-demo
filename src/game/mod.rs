pub mod player;
pub mod tile;

pub fn build(app: &mut crate::AppBuilder) {
    player::build(app);
    tile::build(app);
}

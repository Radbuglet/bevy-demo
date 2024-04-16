pub mod player;

pub fn build(app: &mut crate::AppBuilder) {
    player::build(app);
}

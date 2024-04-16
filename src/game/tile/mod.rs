pub mod data;
pub mod render;

pub fn build(app: &mut crate::AppBuilder) {
    data::build(app);
    render::build(app);
}

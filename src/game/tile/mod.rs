pub mod data;

pub fn build(app: &mut crate::AppBuilder) {
    data::build(app);
}

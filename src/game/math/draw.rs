use macroquad::{color::Color, math::Vec2, shapes::draw_rectangle};

use super::aabb::Aabb;

pub fn stroke_rectangle_aabb(aabb: Aabb, border: f32, color: Color) {
    draw_rectangle_aabb(
        aabb.bottom_right_to(Vec2::new(aabb.max.x, aabb.min.y + border)),
        color,
    );

    draw_rectangle_aabb(
        aabb.top_left_to(Vec2::new(aabb.min.x, aabb.max.y - border)),
        color,
    );

    draw_rectangle_aabb(
        aabb.top_left_by(Vec2::new(0., border))
            .with_size(Vec2::new(border, aabb.h() - border * 2.)),
        color,
    );

    draw_rectangle_aabb(
        aabb.top_left_by(Vec2::new(aabb.w() - border, border))
            .with_size(Vec2::new(border, aabb.h() - border * 2.)),
        color,
    );
}

pub fn draw_rectangle_aabb(aabb: Aabb, color: Color) {
    let aabb = aabb.normalized();
    draw_rectangle(aabb.x(), aabb.y(), aabb.w(), aabb.h(), color);
}

#![warn(clippy::pedantic)]
pub mod network;

type Point2 = mint::Point2<i32>;
type Vector2 = mint::Vector2<i32>;

pub fn point2(x: i32, y: i32) -> Point2 {
    Point2 { x, y}
}

pub fn vector2(x: i32, y: i32) -> Vector2 {
    Vector2 { x, y}
}
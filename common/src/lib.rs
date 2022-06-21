pub mod network;

pub const TILE_SIZE: i32 = 48;
pub const SPRITE_SIZE: i32 = 48;

pub const WALK_SPEED: f64 = 2.5 * TILE_SIZE as f64;
pub const RUN_SPEED: f64 = 5.0 * TILE_SIZE as f64;

type Point2 = mint::Point2<i32>;
type Vector2 = mint::Vector2<i32>;

pub fn point2(x: i32, y: i32) -> Point2 {
    Point2 { x, y }
}

pub fn vector2(x: i32, y: i32) -> Vector2 {
    Vector2 { x, y }
}

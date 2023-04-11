use std::path::{Path, PathBuf};

pub mod network;

pub const TILE_SIZE: i32 = 48;
pub const SPRITE_SIZE: i32 = 48;

pub const WALK_SPEED: f32 = 2.5 * TILE_SIZE as f32;
pub const RUN_SPEED: f32 = 5.0 * TILE_SIZE as f32;

type Point2 = mint::Point2<i32>;
type Vector2 = mint::Vector2<i32>;

pub fn point2(x: i32, y: i32) -> Point2 {
    Point2 { x, y }
}

pub fn vector2(x: i32, y: i32) -> Vector2 {
    Vector2 { x, y }
}

/// Fetches the directory that should be used to store runtime data and assets
pub fn runtime_path(path: impl AsRef<Path>) -> PathBuf {
    let asset_path = if cfg!(debug_assertions) {
        PathBuf::from(std::env::var("RUNTIME_PATH").expect("RUNTIME_PATH must be set when running in debug"))
    } else {
        std::env::current_exe().expect("Failed to retrieve executable path!")
    };
    asset_path.join(path)
}

pub fn client_path(path: impl AsRef<Path>) -> PathBuf {
    runtime_path(PathBuf::from("client").join(path))
}

pub fn server_path(path: impl AsRef<Path>) -> PathBuf {
    runtime_path(PathBuf::from("server").join(path))
}

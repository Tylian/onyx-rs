use std::path::PathBuf;

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

/// Fetches the directory that should be used to store runtime data and assets
pub fn runtime_path(manifest_dir: &str, subfolder: &str) -> PathBuf {
    if cfg!(debug_assertions) {
        // If we're running with debug assertions on, assume that we're
        // in a workspace and that the workspace contains a runtime folder
        let mut path = PathBuf::from(manifest_dir);
        path.pop(); // workspace specific, remove this if need be
        path.extend(["runtime", subfolder]);

        path
    } else {
        // Otherwise, assume that the current directory is the runtime folder
        let mut path = std::env::current_exe().expect("Failed to retrieve executable path!");
        path.pop();

        path
    }
}

#[macro_export]
macro_rules! client_runtime {
    () => {
        $crate::runtime_path(env!("CARGO_MANIFEST_DIR"), "client")
    };
}

#[macro_export]
macro_rules! server_runtime {
    () => {
        $crate::runtime_path(env!("CARGO_MANIFEST_DIR"), "server")
    };
}

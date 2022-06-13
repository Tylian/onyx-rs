#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use game::game_screen;
use title::title_screen;
use macroquad::window::Conf;

mod networking;
mod title;
mod game;
mod player;
mod map;

mod prelude {
    pub use macroquad::prelude::*;
    // pub use glam::*;

    pub use common::network::*;

    pub type GameResult<T> = Result<T, Box<dyn std::error::Error>>;
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Onyx Engine".to_owned(),
        window_width: 960,
        window_height: 720,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #[cfg(debug_assertions)]
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

    let network = title_screen().await;
    game_screen(network).await;
}
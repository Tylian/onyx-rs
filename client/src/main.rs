#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::pedantic)]

use macroquad::window::Conf;

use crate::{game::game_screen, title::title_screen, assets::Assets};

mod assets;
mod game;
mod macros;
mod map;
mod networking;
mod title;

pub type GameResult<T> = Result<T, Box<dyn std::error::Error>>;

fn window_conf() -> Conf {
    Conf {
        window_title: "Onyx Engine".to_owned(),
        window_width: 1600,
        window_height: 900,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut assets = Assets::load().await
        .expect("Could not load assets");

    egui_macroquad::cfg(|ctx| assets.load_egui(ctx));

    let network = title_screen(assets.clone()).await;
    game_screen(network, assets.clone()).await;
}
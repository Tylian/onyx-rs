#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
    let env = env_logger::Env::default()
        .filter_or(env_logger::DEFAULT_FILTER_ENV, if cfg!(debug_assertions) { "debug" } else { "info" });
    env_logger::init_from_env(env);

    let mut assets = Assets::load().await
        .expect("Could not load assets");

    egui_macroquad::cfg(|ctx| assets.load_egui(ctx));

    let network = title_screen(assets.clone()).await;
    game_screen(network, assets.clone()).await;
}
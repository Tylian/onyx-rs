#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;

use env_logger::WriteStyle;
use log::LevelFilter;
use macroquad::window::Conf;

use crate::{assets::Assets, game::game_screen, title::title_screen};

mod assets;
mod game;
mod network;
mod title;
mod ui;
mod utils;
mod data;

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
    #[cfg(debug_assertions)]
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .write_style(WriteStyle::Always)
        .init();

    #[cfg(not(debug_assertions))]
    env_logger::builder()
        .filter_level(LevelFilter::Warn)
        .init();

    let assets = Assets::load().await.expect("Could not load assets");
    let assets = Rc::new(assets);

    let (client_id, network) = title_screen(Rc::clone(&assets)).await;
    game_screen(network, client_id, Rc::clone(&assets)).await;
}

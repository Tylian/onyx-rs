#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;

use macroquad::window::Conf;

use crate::{assets::Assets, game::game_screen, title::title_screen};

mod assets;
mod game;
mod map;
mod networking;
mod title;
mod ui;
mod utils;

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
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    #[cfg(not(debug_assertions))]
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let assets = Assets::load().await.expect("Could not load assets");
    let assets = Rc::new(assets);

    let network = title_screen(Rc::clone(&assets)).await;
    game_screen(network, Rc::clone(&assets)).await;
}

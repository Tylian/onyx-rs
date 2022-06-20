#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use glam::Vec2;
use macroquad::{window::Conf, prelude::*};

use crate::{game::game_screen, title::title_screen, assets::Assets};

mod assets;
mod game;
mod macros;
mod map;
mod networking;
mod title;
mod ui;

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

// ping pong animation
fn ping_pong(t: f64, frames: u32) -> u32 {
    let frame = ((frames - 1) as f64 * 2.0 * t) as u32;
    if frame > frames {
        frames - frame
    } else {
        frame
    }
}

fn draw_text_shadow(text: &str, position: Vec2, params: TextParams) {
    let outlines = &[
        (1.0, 0.0).into(),
        (-1.0, 0.0).into(),
        (0.0, 1.0).into(),
        (0.0, -1.0).into(),
        (-1.0, -0.0).into(),
        (-1.0, 1.0).into(),
        (1.0, -1.0).into(),
        (1.0, 1.0).into(),
    ];

    let outline_param = TextParams {
        color: Color::new(0.0, 0.0, 0.0, 0.5),
        ..params
    };

    for outline in outlines {
        let position = position + *outline;
        draw_text_ex(text, position.x, position.y, outline_param);
    }

    draw_text_ex(text, position.x, position.y, params);
}
use std::{fs, path::PathBuf, rc::Rc};

use macroquad::{color, prelude::*};
use onyx_common::network::ClientMessage;
use serde::{Deserialize, Serialize};

use crate::{
    assets::Assets,
    networking::{NetworkClient, NetworkStatus},
    ui::sprite_preview,
};

#[derive(Serialize, Deserialize)]
struct Settings {
    address: String,
    name: String,
    sprite: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            address: "66.228.47.52:3042".to_owned(),
            name: "Player".to_owned(),
            sprite: 0,
        }
    }
}

struct UiState {
    settings: Settings,
    network: Option<NetworkClient>,
}

fn draw_ui(ctx: &egui::Context, state: &mut UiState, assets: &Assets) {
    use egui::*;

    let login_window = Window::new("Login")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false);

    let is_connecting = state.network.is_some();
    let time = get_time();

    login_window.show(ctx, |ui| {
        ui.add_enabled_ui(!is_connecting, |ui| {
            Grid::new("login").num_columns(2).show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut state.settings.name);
                ui.end_row();
                ui.label("Server");
                ui.text_edit_singleline(&mut state.settings.address);
                ui.end_row();
                ui.label("Sprite:");
                ui.horizontal_centered(|ui| {
                    ui.add(
                        DragValue::new(&mut state.settings.sprite)
                            .clamp_range(0u32..=55u32)
                            .speed(0.1),
                    );

                    sprite_preview(ui, &assets.sprites.egui, time, state.settings.sprite);
                });
                ui.end_row();
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Login").clicked() {
                    let mut network = NetworkClient::new();
                    network.connect(state.settings.address.clone());

                    // ! TODO
                    network.send(ClientMessage::Hello(state.settings.name.clone(), state.settings.sprite));
                    state.network = Some(network);
                }
                if is_connecting {
                    ui.spinner();
                }
            });
        });
    });
}

pub async fn title_screen(assets: Rc<Assets>) -> NetworkClient {
    let path = PathBuf::from("./settings.bin");
    let settings = fs::read(path)
        .ok()
        .and_then(|bytes| bincode::deserialize(&bytes).ok())
        .unwrap_or_default();

    let mut state = UiState {
        settings,
        network: None,
    };

    loop {
        // update
        egui_macroquad::ui(|egui_ctx| draw_ui(egui_ctx, &mut state, &assets));

        let is_online = state
            .network
            .as_ref()
            .map_or(false, |n| n.status() == NetworkStatus::Connected);

        if is_online {
            let written = bincode::serialize(&state.settings)
                .ok()
                .and_then(|bytes| fs::write("./settings.bin", bytes).ok())
                .is_some();

            if !written {
                println!("Couldn't write settings, just fyi");
            }

            return state.network.unwrap();
        }

        // draw
        clear_background(color::BLACK);
        egui_macroquad::draw();

        next_frame().await;
    }
}

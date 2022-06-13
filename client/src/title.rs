use macroquad::prelude::*;
use common::network::ClientMessage;

use crate::networking::{Networking, NetworkStatus};

struct UiState {
    address: String,
    name: String,
    sprite: u32,
    network: Option<Networking>
}

fn draw_ui(ctx: &egui::Context, state: &mut UiState) {
    use egui::*;

    let login_window = Window::new("Login")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false);

    let is_connecting = state.network.is_some();

    login_window.show(ctx, |ui| {
        ui.add_enabled_ui(!is_connecting, |ui| {
            Grid::new("login").num_columns(2).show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut state.name);
                ui.end_row();
                ui.label("Sprite:");
                ui.add(DragValue::new(&mut state.sprite).clamp_range(0u32..=6u32));

                ui.end_row();
                ui.label("Server");
                ui.text_edit_singleline(&mut state.address);
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Login").clicked() {
                    let mut network = Networking::new();
                    network.connect(state.address.clone());

                    // ! TODO
                    network.send(ClientMessage::Hello(state.name.clone(), state.sprite));
                    state.network = Some(network);
                }
                if is_connecting {
                    ui.spinner();
                }
            });
        });
    });  
}

pub async fn title_screen() -> Networking {
    let mut state = UiState {
        address: "127.0.0.1:3042".to_owned(),
        name: "Namda".to_owned(),
        sprite: 0,
        network: None,
    };

    loop {
        // update
        egui_macroquad::ui(|egui_ctx| draw_ui(egui_ctx, &mut state));

        let is_online = state.network.as_ref()
            .map_or(false, |n| n.status() == NetworkStatus::Connected);

        if is_online {
            return state.network.unwrap();
        }

        // draw
        clear_background(BLACK);
        egui_macroquad::draw();

        next_frame().await;
    }
}

use macroquad::prelude::*;
use common::network::ClientMessage;

use crate::{networking::{NetworkClient, NetworkStatus}, assets::Assets};

const SPRITE_SIZE: f32 = 48.;

struct UiState {
    address: String,
    name: String,
    sprite: u32,
    network: Option<NetworkClient>
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
                ui.text_edit_singleline(&mut state.name);
                ui.end_row();
                ui.label("Server");
                ui.text_edit_singleline(&mut state.address);
                ui.end_row();
                ui.label("Sprite:");
                ui.horizontal_centered(|ui| {
                    ui.add(DragValue::new(&mut state.sprite).clamp_range(0u32..=55u32));
                    let texture = assets.egui.sprites.as_ref().unwrap();
    
                    let sprite_x = (state.sprite as f64 % 4.0) * 3.0;
                    let sprite_y = (state.sprite as f64 / 4.0).floor() * 4.0;
    
                    // walk left and right 
                    let offset_x = (((time / 0.25).floor() % 4.0).floor() - 1.0).abs();
                    let offset_y = ((time / 4.).floor() % 4.).floor();
    
                    let p = vec2((sprite_x + offset_x) as f32 * SPRITE_SIZE, (sprite_y + offset_y) as f32 * SPRITE_SIZE) / texture.size_vec2();
                    let size = vec2(SPRITE_SIZE, SPRITE_SIZE) / texture.size_vec2();
                    let sprite = Image::new(texture, (SPRITE_SIZE, SPRITE_SIZE))
                        .uv(Rect::from_min_size(p.to_pos2(), size));
                    ui.add(sprite);
                });
                ui.end_row();
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Login").clicked() {
                    let mut network = NetworkClient::new();
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

pub async fn title_screen(assets: Assets) -> NetworkClient {
    // let mut state = UiState {
    //     address: "127.0.0.1:3042".to_owned(),
    //     name: "Namda".to_owned(),
    //     sprite: 5,
    //     network: None,
    // };

    let mut state = UiState {
        address: "66.228.47.52:3042".to_owned(),
        name: "Player".to_owned(),
        sprite: 0,
        network: None,
    };

    loop {
        // update
        egui_macroquad::ui(|egui_ctx| draw_ui(egui_ctx, &mut state, &assets));

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

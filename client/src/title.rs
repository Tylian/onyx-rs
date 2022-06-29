use std::{path::PathBuf, rc::Rc};

use anyhow::Result;
use common::network::{ClientId, ClientMessage, ServerMessage};
use macroquad::{color, prelude::*};
use message_io::node::StoredNetEvent;
use serde::{Deserialize, Serialize};

use crate::{assets::Assets, network::Network};

#[derive(Serialize, Deserialize)]
struct Settings {
    address: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: Option<String>,
}

impl Settings {
    fn path() -> PathBuf {
        let mut path = common::client_runtime!();
        path.push("settings.toml");
        path
    }

    fn load() -> Result<Self> {
        let contents = std::fs::read_to_string(Settings::path())?;
        Ok(toml::from_str(&contents)?)
    }

    fn save(&self) -> Result<()> {
        let contents = toml::to_string_pretty(&self)?;
        std::fs::write(Settings::path(), &contents)?;

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            address: "66.228.47.52:20371".to_owned(),
            username: String::new(),
            password: None,
        }
    }
}

struct UiState {
    username: String,
    password: String,
    save_password: bool,
    character_name: String,
    network: Network,
    loading: bool,
    error: Option<String>,
    tab: UiTab,
}

#[derive(Copy, Clone, PartialEq)]
enum UiTab {
    Login,
    Create,
}

fn draw_login(ui: &mut egui::Ui, state: &mut UiState, _assets: &Assets) {
    use egui::*;

    Grid::new("login").num_columns(2).show(ui, |ui| {
        ui.label("Username:");
        ui.text_edit_singleline(&mut state.username);
        ui.end_row();

        ui.label("Password:");
        ui.add(TextEdit::singleline(&mut state.password).password(true));
        ui.end_row();

        ui.add_space(0.0);
        ui.checkbox(&mut state.save_password, "Save password?");
        ui.end_row();
    });
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Login").clicked() {
            state.error = None;
            state.loading = true;
            state.network.send(ClientMessage::Login {
                username: state.username.clone(),
                password: state.password.clone(),
            });
        }
        if state.loading {
            ui.spinner();
        }
        if let Some(error) = state.error.as_ref() {
            ui.colored_label(Color32::RED, error);
        }
    });
}

fn draw_create(ui: &mut egui::Ui, state: &mut UiState, _assets: &Assets) {
    use egui::*;

    Grid::new("create").num_columns(2).show(ui, |ui| {
        ui.label("Username:");
        ui.text_edit_singleline(&mut state.username);
        ui.end_row();

        ui.label("Password:");
        ui.add(TextEdit::singleline(&mut state.password).password(true));
        ui.end_row();

        ui.label("Character name:");
        ui.text_edit_singleline(&mut state.character_name);
        ui.end_row();
    });
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Create character").clicked() {
            state.error = None;
            state.loading = true;
            state.network.send(ClientMessage::CreateAccount {
                username: state.username.clone(),
                password: state.password.clone(),
                character_name: state.character_name.clone(),
            });
        }
        if state.loading {
            ui.spinner();
        }
        if let Some(error) = state.error.as_ref() {
            ui.colored_label(Color32::RED, error);
        }
    });
}

fn draw_ui(ctx: &egui::Context, state: &mut UiState, assets: &Assets) {
    use egui::*;

    let login_window = Window::new("Login")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false);

    login_window.show(ctx, |ui| {
        ui.add_enabled_ui(!state.loading, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.tab, UiTab::Login, "Login");
                ui.selectable_value(&mut state.tab, UiTab::Create, "Create character");
            });
            ui.separator();
            match state.tab {
                UiTab::Login => draw_login(ui, state, assets),
                UiTab::Create => draw_create(ui, state, assets),
            }
        });
    });
}

pub async fn title_screen(assets: Rc<Assets>) -> (ClientId, Network) {
    let settings = Settings::load().unwrap_or_default();

    let mut state = UiState {
        network: Network::connect(&settings.address),
        error: None,
        tab: UiTab::Login,
        loading: true,
        username: settings.username,
        save_password: settings.password.is_some(),
        password: settings.password.unwrap_or_default(),
        character_name: String::new(),
    };

    loop {
        // network
        if let Some(event) = state.network.try_receive() {
            match event.network() {
                StoredNetEvent::Connected(_, _) => {
                    state.loading = false;
                }
                StoredNetEvent::Accepted(_, _) => unreachable!(),
                StoredNetEvent::Message(_, bytes) => {
                    let message = rmp_serde::from_slice(&bytes).unwrap();
                    log::debug!("{message:?}");

                    match message {
                        ServerMessage::JoinGame(client_id) => {
                            let settings = Settings {
                                address: settings.address,
                                username: state.username,
                                password: state.save_password.then(|| state.password),
                            };

                            if let Err(e) = settings.save() {
                                println!("Couldn't write settings, just fyi: {:?}", e);
                            }

                            return (client_id, state.network);
                        }
                        ServerMessage::FailedJoin(reason) => {
                            state.error = Some(reason.to_string());
                            state.loading = false;
                        }
                        _ => unreachable!(),
                    }
                }
                StoredNetEvent::Disconnected(_) => {
                    state.loading = true;
                    state.network = Network::connect(&settings.address);
                }
            }
        }

        // update
        egui_macroquad::ui(|egui_ctx| draw_ui(egui_ctx, &mut state, &assets));

        // draw
        clear_background(color::BLACK);
        egui_macroquad::draw();

        next_frame().await;
    }
}

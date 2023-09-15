use std::{path::PathBuf};

use common::network::client::Packet;
use message_io::node::StoredNetEvent;
use notan::{
    egui::{self, *},
    log,
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{game::GameState, network::Network, state::{UpdateContext, DrawContext, SetupContext, EventContext}};

use super::State;

pub struct TitleState {
    settings: Settings,
    network: Option<Network>,

    loading: bool,
    tab: UiTab,

    username: String,
    password: String,
    save_password: bool,
    character_name: String,
    error: Option<String>,
}

impl TitleState {
    pub fn new_erased(_ctx: &mut SetupContext) -> Box<dyn State> {
        let settings = Settings::load().unwrap_or_default();

        Box::new(Self {
            network: Some(Network::connect(&settings.address)),

            loading: true,
            tab: UiTab::Login,

            username: settings.username.clone(),
            save_password: settings.password.is_some(),
            password: settings.password.clone().unwrap_or_default(),
            character_name: String::new(),
            error: None,
            settings,
        })
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        Window::new("Login")
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!self.loading, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.tab, UiTab::Login, "Login");
                        ui.selectable_value(&mut self.tab, UiTab::Create, "Create character");
                    });
                    ui.separator();
                    match self.tab {
                        UiTab::Login => self.ui_login(ui),
                        UiTab::Create => self.ui_create(ui),
                    }
                });
            });

        //     // todo
        //     if state.dialog.is_some() {
        //         let resp = dialog(ctx, |ui| {
        //             ui.heading("\u{2139} Hello uwu??");

        //             ui.separator();
        //             ui.label(state.dialog.as_ref().unwrap());
        //             ui.separator();

        //             ui.horizontal(|ui| {
        //                 ui.scope(|ui| {
        //                     let bg_fill = Color32::DARK_GREEN;
        //                     ui.visuals_mut().widgets.inactive.bg_fill = bg_fill;
        //                     ui.visuals_mut().widgets.active.bg_fill = bg_fill;
        //                     ui.visuals_mut().widgets.hovered.bg_fill = bg_fill;

        //                     if ui.button("Okay?").clicked() {
        //                         state.dialog = None;
        //                     }
        //                 });

        //                 ui.scope(|ui| {
        //                     let bg_fill = Color32::DARK_RED;
        //                     ui.visuals_mut().widgets.inactive.bg_fill = bg_fill;
        //                     ui.visuals_mut().widgets.active.bg_fill = bg_fill;
        //                     ui.visuals_mut().widgets.hovered.bg_fill = bg_fill;

        //                     if ui.button("No???").clicked() {
        //                         state.dialog = None;
        //                     }
        //                 });
        //             });
        //         });

        //         if resp.response.clicked() {
        //             state.dialog = None;
        //         }
        //     }
    }

    pub fn ui_login(&mut self, ui: &mut egui::Ui) {
        Grid::new("login").num_columns(2).show(ui, |ui| {
            ui.label("Username:");
            ui.text_edit_singleline(&mut self.username);
            ui.end_row();

            ui.label("Password:");
            ui.add(TextEdit::singleline(&mut self.password).password(true));
            ui.end_row();

            ui.add_space(0.0);
            ui.checkbox(&mut self.save_password, "Save password?");
            ui.end_row();
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Login").clicked() {
                if let Some(network) = self.network.as_ref() {
                    self.error = None;
                    self.loading = true;
                    network.send(&Packet::Login {
                        username: self.username.clone(),
                        password: self.password.clone(),
                    });
                }
            }
            if self.loading {
                ui.spinner();
            }
            if let Some(error) = self.error.as_ref() {
                ui.colored_label(Color32::RED, error);
            }
        });
    }

    pub fn ui_create(&mut self, ui: &mut egui::Ui) {
        Grid::new("create").num_columns(2).show(ui, |ui| {
            ui.label("Username:");
            ui.text_edit_singleline(&mut self.username);
            ui.end_row();

            ui.label("Password:");
            ui.add(TextEdit::singleline(&mut self.password).password(true));
            ui.end_row();

            ui.label("Character name:");
            ui.text_edit_singleline(&mut self.character_name);
            ui.end_row();
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Create character").clicked() {
                if let Some(network) = self.network.as_ref() {
                    self.error = None;
                    self.loading = true;
                    network.send(&Packet::CreateAccount {
                        username: self.username.clone(),
                        password: self.password.clone(),
                        character_name: self.character_name.clone(),
                    });
                }
            }
            if self.loading {
                ui.spinner();
            }
            if let Some(error) = self.error.as_ref() {
                ui.colored_label(Color32::RED, error);
            }
        });
    }
}

impl State for TitleState {
    fn draw(&mut self, ctx: &mut DrawContext) {
        let mut output = ctx.plugins.egui(|egui_ctx| self.ui(egui_ctx));
        output.clear_color(Color::BLACK);
        
        ctx.gfx.render(&output);
    }

    fn update(&mut self, ctx: &mut UpdateContext) {
        let Some(network) = self.network.as_mut() else {
            return;
        };

        if let Some(event) = network.try_receive() {
            use common::network::server::Packet;

            match event.network() {
                StoredNetEvent::Connected(_, ok) => {
                    if ok {
                        self.loading = false;
                        self.error = None;
                    } else {
                        self.error = Some(String::from("could not connect"));
                        self.loading = true;
                        network.stop();
                        self.network.replace(Network::connect(&self.settings.address));
                    }
                }
                StoredNetEvent::Accepted(_, _) => unreachable!(),
                StoredNetEvent::Message(_, bytes) => {
                    let message = rmp_serde::from_slice(&bytes).unwrap();
                    log::debug!("{message:?}");

                    match message {
                        Packet::JoinGame(entity) => {
                            let settings = Settings {
                                address: self.settings.address.clone(),
                                username: self.username.clone(),
                                password: self.save_password.then_some(self.password.clone()),
                            };

                            if let Err(e) = settings.save() {
                                println!("Couldn't write settings, just fyi: {:?}", e);
                            }

                            let network = self.network.take().unwrap();
                            ctx.next_state_fn = Some(Box::new(move |ctx| {
                                    Box::new(GameState::new(entity, network, ctx))
                            }));
                        }
                        Packet::FailedJoin(reason) => {
                            self.error = Some(reason.to_string());
                            self.loading = false;
                        }
                        _ => unreachable!(),
                    }
                }
                StoredNetEvent::Disconnected(_) => {
                    self.error = Some(String::from("disconnected"));
                    self.loading = true;
                    network.stop();
                    self.network.replace(Network::connect(&self.settings.address));
                }
            }
        }
    }

    fn event(&mut self, ctx: &mut EventContext) {
        use notan::Event;
        if ctx.event == Event::Exit {
            if let Some(network) = self.network.as_mut() {
                network.stop();
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum UiTab {
    Login,
    Create,
}

#[derive(Serialize, Deserialize)]
struct Settings {
    address: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: Option<String>,
}

impl Settings {
    pub fn path() -> PathBuf {
        common::client_path("settings.toml")
    }

    pub fn load() -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(Self::path())?;
        Ok(toml::from_str(&contents)?)
    }

    fn save(&self) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(&self)?;
        std::fs::write(Self::path(), contents)?;

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

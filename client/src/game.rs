use common::network::{ClientId, ChatMessage, ServerMessage, ClientMessage, Direction, MapLayer};
use macroquad::prelude::*;

use crate::assets::Assets;
use crate::networking::{NetworkClient, NetworkStatus};
use crate::map::{Map, Tile};
use self::player::Player;

mod player;

const MOVEMENT_SPEED: f64 = 1.0 / 5.0;
const TILE_WIDTH: f32 = 48.0;
const TILE_HEIGHT: f32 = 48.0;

#[inline(always)]
fn ivec2_to_egui(ivec: IVec2) -> egui::Vec2 {
    egui::Vec2::new(ivec.x as f32, ivec.y as f32)
}

#[inline(always)]
fn egui_to_ivec2(pos: egui::Pos2) -> IVec2 {
    ivec2(pos.x as i32, pos.y as i32)
}

struct UiState {
    chat_message: String,
    chat_messages: Vec<ChatMessage>,
    layer: MapLayer,
    coords: IVec2,
    is_autotile: bool,
    last_tile: Option<(MouseButton, IVec2)>,
    map_editor: bool,
    hovering_ui: bool
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            chat_message: Default::default(),
            chat_messages: Default::default(),
            layer: MapLayer::Ground,
            coords: Default::default(),
            is_autotile: Default::default(),
            last_tile: Default::default(),
            map_editor: Default::default(),
            hovering_ui: Default::default(),
        }
    }
}

struct GameState {
    assets: Assets,
    network: NetworkClient,
    my_id: Option<ClientId>,
    players: Vec<Player>,
    map: Map,
    ui_state: UiState,
    movement_lock: f64,
    time: f64,
}

impl GameState {
    fn new(network: NetworkClient, assets: Assets) -> Self {
        Self {
            assets,
            network,
            my_id: Default::default(),
            players: Default::default(),
            map: Map::new(20, 15),
            ui_state: Default::default(),
            movement_lock: Default::default(),
            time: get_time(),
        }
    }

    fn player(&self, id: ClientId) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    fn player_mut(&mut self, id: ClientId) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    fn sort_players(&mut self) {
        self.players.sort_by_cached_key(|player| player.position.y);
    }

    fn me(&self) -> Option<&Player> {
        self.my_id.and_then(|my_id| self.players.iter().find(|p| p.id == my_id))
    }

    fn me_mut(&mut self) -> Option<&mut Player> {
        self.my_id.and_then(|my_id| self.players.iter_mut().find(|p| p.id == my_id))
    }

    fn process_message(&mut self, text: String) {
        if text.starts_with("/mapeditor") {
            self.ui_state.map_editor = true;
        } else {
            self.network.send(ClientMessage::Message(text));
        }
    }

    fn update(&mut self) {
        self.update_time();

        self.update_network();
        egui_macroquad::ui(|ctx| {
            self.update_ui(ctx);
            self.ui_state.hovering_ui = ctx.wants_pointer_input();
        });
        
        for player in self.players.iter_mut() {
            player.update(self.time);
        }

        self.update_input();
    }

    fn update_time(&mut self) {
        self.time = get_time();
    }

    fn update_network(&mut self) {
        if self.network.status() != NetworkStatus::Connected {
            return;
        }

        let time = self.time;
        while let Some(message) = self.network.try_recv() {
            println!("{:?}", message);
            match message {
                ServerMessage::Hello(client_id) => {
                    self.my_id = Some(client_id);
                }
                ServerMessage::PlayerJoined(client_id, player_data) => {
                    self.players.push(Player::from_network(client_id, player_data));
                    self.sort_players();
                }
                ServerMessage::PlayerLeft(client_id) => {
                    // self.players.retain(|p| p.id != client_id);
                    let idx = self.players.iter().position(|p| p.id == client_id);
                    if let Some(idx) = idx {
                        self.players.swap_remove(idx);
                        self.sort_players();
                    }
                }
                ServerMessage::PlayerMoved { client_id, position, direction } => {
                    let from = IVec2::from(position) + direction.reverse().offset().into();
                    let mut player = self.player_mut(client_id).unwrap();
                    player.position = position.into();
                    player.direction = direction;

                    player.set_tween(from, time, MOVEMENT_SPEED);
                    self.sort_players();
                }
                ServerMessage::Message(message) => {
                    self.ui_state.chat_messages.push(message);
                },
                ServerMessage::ChangeTile { position, layer, tile: uv, is_autotile }  => {
                    let tile = self.map.tile_mut(layer, position.into());
                    if let Some(tile) = tile {
                        *tile = match uv {
                            Some(uv) if is_autotile => Tile::autotile(uv.into()),
                            Some(uv) => Tile::basic(uv.into()),
                            None => Tile::Empty
                        };
                        self.map.update_autotiles();
                    }
                },
                ServerMessage::ChangeMap(remote) => {
                    self.map = remote.into();
                }
            }
        }
    }

    fn update_ui(&mut self, ctx: &egui::Context) {
        use egui::{*, style::Margin};
        use egui_extras::{StripBuilder, Size};

        let chat_window = Window::new("💬 Chat")
            .resizable(true)
            .default_pos([7.0, 522.0])
            .default_size([367.0, 147.0])
            .min_height(125.0);

        let mut text: Option<Response> = None;
        let mut button: Option<Response> = None;

        // 7 522 386 708
        chat_window.show(&ctx, |ui| {
            let bottom_height = ui.spacing().interact_size.y;
            StripBuilder::new(ui)
                .size(Size::remainder().at_least(100.))
                .size(Size::exact(6.))
                .size(Size::exact(bottom_height))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .stick_to_bottom()
                            .show(ui, |ui| {
                                for message in &self.ui_state.chat_messages {
                                    match message {
                                        ChatMessage::Server(text) => {
                                            ui.colored_label(Color32::YELLOW, format!("[Server] {}\n", text));
                                        },
                                        ChatMessage::Say(text) => {
                                            ui.colored_label(Color32::WHITE, format!("[Say] {}\n", text));
                                        }
                                    };
                                }
                            });
                    });
                    strip.cell(|ui| { ui.separator(); });
                    strip.strip(|builder| {
                        builder
                            .size(Size::exact(40.))
                            .size(Size::remainder())
                            .size(Size::exact(40.))
                            .horizontal(|mut strip| {
                                strip.cell(|ui| {
                                    ui.colored_label(Color32::WHITE, "Say:");
                                });
                                strip.cell(|ui| {
                                    text = Some(ui.text_edit_singleline(&mut self.ui_state.chat_message));
                                });
                                strip.cell(|ui| {
                                    button = Some(ui.button("Send"));
                                });
                            });
                    });
                });

            if let Some((text, button)) = text.zip(button) {
                if (text.lost_focus() && ui.input().key_pressed(Key::Enter)) || button.clicked() {
                    let message = std::mem::take(&mut self.ui_state.chat_message);
                    self.process_message(message);
                    text.request_focus();
                }
            }
        });

        let map_editor = Window::new("📝 Map Editor");
        if self.ui_state.map_editor {
            map_editor.show(ctx, |ui| {
                menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Save").clicked() {
                            let data = self.map.clone().into();
                            self.network.send(ClientMessage::SaveMap(data));
                            self.ui_state.map_editor = false;
                        }
                        if ui.button("Exit").clicked() {
                            self.network.send(ClientMessage::RequestMap);
                            self.ui_state.map_editor = false;
                        }
                    });
                    ui.menu_button("Layer", |ui| {
                        ui.radio_value(&mut self.ui_state.layer, MapLayer::Ground, "Ground");
                        ui.radio_value(&mut self.ui_state.layer, MapLayer::Mask, "Mask");
                        ui.radio_value(&mut self.ui_state.layer, MapLayer::Fringe, "Fringe");
                        ui.separator();
                        ui.checkbox(&mut self.ui_state.is_autotile, "Is autotile?");
                    });
                });
                
                if let Some(texture) = self.assets.egui.tileset.as_ref() {
                    // let p: Vec2 = vec2(self.coords.x as f32 * TILE_WIDTH, self.coords.y as f32 * TILE_HEIGHT) / texture.size_vec2();
                    // let size = vec2(TILE_WIDTH, TILE_HEIGHT) / texture.size_vec2();
                    // let tile = Image::new(texture, (TILE_WIDTH, TILE_HEIGHT))
                    //     .uv(Rect::from_min_size(p.to_pos2(), size));
                    // ui.add(tile);
    
                    let scroll_area = ScrollArea::both();
                        // .auto_shrink([false, false])
                        // .max_height(8. * 48.);
                    scroll_area.show_viewport(ui, |ui, viewport| {
                        let image = Image::new(texture, texture.size_vec2())
                            .sense(Sense::click());
    
                        let clip_rect = ui.clip_rect();
    
                        let response = ui.add(image);
                        if response.clicked() {
                            let pos = response.interact_pointer_pos().expect("Pointer position shouldn't be None");
                            let offset = viewport.left_top() + (pos - clip_rect.left_top()); // weird order just to make it typecheck lol
                            self.ui_state.coords = egui_to_ivec2(offset) / 48;
                        }
    
                        let pos = (clip_rect.left_top() - viewport.left_top()) + ivec2_to_egui(self.ui_state.coords * 48);
                        let rect = Rect::from_min_size(pos.to_pos2(), Vec2::new(48., 48.));
    
                        let painter = ui.painter();
                        painter.rect_stroke(rect, 0., ui.visuals().window_stroke());
    
                        response
                    });
                }
                
            });
        }

        /*egui::Window::new("📝 Memory")
        .resizable(false)
        .show(&egui_ctx, |ui| {
            egui_ctx.memory_ui(ui);
        });*/
    }

    fn update_input(&mut self) {
        if self.my_id.is_some() {
            if self.movement_lock <= self.time {
                let movement = if is_key_down(KeyCode::Up) {
                    Some(Direction::North)
                } else if is_key_down(KeyCode::Down) {
                    Some(Direction::South)
                } else if is_key_down(KeyCode::Left) {
                    Some(Direction::West)
                } else if is_key_down(KeyCode::Right) {
                    Some(Direction::East)
                } else {
                    None
                };

                if let Some(direction) = movement {
                    let new_position = self.me().unwrap().position + direction.offset().into();
                    if self.map.valid(new_position) {
                        let time = self.time;
                        let me = self.me_mut().unwrap();
                        let from = me.position;
                        me.position = new_position;
                        me.direction = direction;

                        me.set_tween(from, time, MOVEMENT_SPEED);

                        self.network.send(ClientMessage::Move(direction));
                        self.movement_lock = self.time + MOVEMENT_SPEED;
                    }
                }
            }
        }
        
        let mouse_button = if self.ui_state.hovering_ui {
            None
        } else {
            if is_mouse_button_down(MouseButton::Left) {
                Some(MouseButton::Left)
            } else if is_mouse_button_down(MouseButton::Right) {
                Some(MouseButton::Right)
            } else {
                None
            }
        };

        if let Some(mouse_button) = mouse_button {
            let mouse_position = Vec2::from(mouse_position()).as_i32();
            let tile_position = mouse_position / 48;

            let current_tile = (mouse_button, tile_position);

            if self.ui_state.last_tile != Some(current_tile) && self.ui_state.map_editor {
                let tile = self.map.tile_mut(self.ui_state.layer, tile_position).unwrap();
                *tile = match mouse_button {
                    MouseButton::Left if self.ui_state.is_autotile => Tile::autotile(self.ui_state.coords),
                    MouseButton::Left => Tile::basic(self.ui_state.coords),
                    MouseButton::Right => Tile::empty(),
                    _ => unreachable!()
                };

                self.map.update_autotiles();

                self.ui_state.last_tile = Some(current_tile);
            }
        }
    }

    fn draw(&mut self) {
        clear_background(BLACK);

        self.map.draw(MapLayer::Ground, &self.assets);
        self.map.draw(MapLayer::Mask, &self.assets);

        for player in &self.players {
            player.draw(self.time, &self.assets);
        }

        self.map.draw(MapLayer::Fringe, &self.assets);
    }
}

pub async fn game_screen(network: NetworkClient, assets: Assets) {
    let mut state = GameState::new(network, assets);

    loop {
        state.update();
        state.draw();

        egui_macroquad::draw();

        next_frame().await;
    }
}
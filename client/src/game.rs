
use std::{collections::HashMap, hash::Hash};

use common::network::{ClientId, ChatChannel, ServerMessage, ClientMessage, Direction};

use crate::{prelude::*, networking::{Networking, NetworkStatus}, player::Player, map::{Map, Tile}};

const MOVEMENT_SPEED: f64 = 1.0 / 5.0;
const TILE_WIDTH: f32 = 48.0;
const TILE_HEIGHT: f32 = 48.0;

pub struct GameState {
    chat_message: String,
    chat_messages: Vec<(ChatChannel, String)>,
    movement_lock: f64,
    my_id: Option<ClientId>,
    players: Vec<Player>,
    network: Networking,
    map: Map,
    layer: MapLayer,
    coords: IVec2,
    is_autotile: bool,
    last_tile: Option<IVec2>
}

impl GameState {
    fn with_network(network: Networking) -> Self {
        Self {
            chat_message: Default::default(),
            chat_messages: Default::default(),
            movement_lock: Default::default(),
            my_id: Default::default(),
            players: Default::default(),
            network,
            map: Map::new(20, 15),
            layer: MapLayer::Ground,
            coords: ivec2(0, 0),
            is_autotile: false,
            last_tile: None
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


    fn update_network(&mut self, time: f64) {
        if self.network.status() != NetworkStatus::Connected {
            return;
        }

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
                ServerMessage::PlayerMoved(client_id, to, direction) => {
                    let from = IVec2::from(to) + direction.reverse().offset().into();
                    let mut player = self.player_mut(client_id).unwrap();
                    player.position = to.into();
                    player.direction = direction;

                    player.set_tween(from, time, MOVEMENT_SPEED);
                    self.sort_players();
                }
                ServerMessage::Message(channel, message) => {
                    self.chat_messages.push((channel, message))
                },
                ServerMessage::ChangeTile(position, layer, uv, is_autotile) => {
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
            }
        }
    }
    fn update_ui(&mut self, ctx: &egui::Context, assets: &Assets) {
        use egui::{*, style::Margin};

        let chat_window = Window::new("💬 Chat")
            .resizable(true)
            .default_pos([7.0, 522.0])
            .default_size([367.0, 147.0])
            .min_height(125.0);

        // 7 522 386 708
        chat_window.show(&ctx, |ui| {
            TopBottomPanel::bottom("chat_bottom")
                .frame(Frame::none().inner_margin(Margin::symmetric(8.0, 2.0)))
                .show_inside(ui, |ui| {
                    ui.separator();
                    ui.with_layout(Layout::right_to_left(), |ui| {
                        let button = ui.button("Send");
                        let text = ui.add_sized(
                            ui.available_size(),
                            TextEdit::singleline(&mut self.chat_message),
                        );

                        if (text.lost_focus() && ui.input().key_pressed(Key::Enter))
                            || button.clicked()
                        {
                            let text = std::mem::take(&mut self.chat_message);
                            self.network.send(ClientMessage::Message(text));
                        }
                    });
                });

            CentralPanel::default().show_inside(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom()
                    .show(ui, |ui| {
                        for (channel, message) in &self.chat_messages {
                            let color = match channel {
                                ChatChannel::Server => Color32::YELLOW,
                                ChatChannel::Say => Color32::WHITE,
                            };

                            let prefix = match channel {
                                ChatChannel::Server => "[Server] ",
                                ChatChannel::Say => "",
                            };
                            ui.colored_label(color, format!("{}{}", prefix, message));
                        }
                    });
            });
        });

        let map_editor = Window::new("📝 Map Editor");

        map_editor.show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.layer, MapLayer::Ground, "Ground");
                ui.radio_value(&mut self.layer, MapLayer::Mask, "Mask");
                ui.radio_value(&mut self.layer, MapLayer::Fringe, "Fringe");
            });
            ui.checkbox(&mut self.is_autotile, "Is autotile?");
            ui.horizontal(|ui| {
                ui.label("Texture: ");
                ui.add(DragValue::new(&mut self.coords.x).speed(0.1).clamp_range(0..=16).prefix("x: "));
                ui.add(DragValue::new(&mut self.coords.y).speed(0.1).clamp_range(0..=12).prefix("y: "));
            });
            if let Some(texture) = assets.egui.tileset.as_ref() {
                let p: Vec2 = vec2(self.coords.x as f32 * TILE_WIDTH, self.coords.y as f32 * TILE_HEIGHT) / texture.size_vec2();
                let size = vec2(TILE_WIDTH, TILE_HEIGHT) / texture.size_vec2();
                let tile = Image::new(texture, (TILE_WIDTH, TILE_HEIGHT))
                    .uv(Rect::from_min_size(p.to_pos2(), size));
                ui.add(tile);
            }
        });

        /*egui::Window::new("📝 Memory")
        .resizable(false)
        .show(&egui_ctx, |ui| {
            egui_ctx.memory_ui(ui);
        });*/
    }
}

pub struct Assets {
    pub tileset: Texture2D,
    pub sprites: Texture2D,
    pub font: Font,
    pub egui: EguiAssets
}

#[derive(Default)]
pub struct EguiAssets {
    pub tileset: Option<egui::TextureHandle>,
    pub sprites: Option<egui::TextureHandle>,
}

impl Assets {
    async fn load() -> GameResult<Self> {
        Ok(Self {
            tileset: load_texture("assets/Outside_A2.png").await?,
            sprites: load_texture("assets/Actor1.png").await?,
            font: load_ttf_font("assets/LiberationMono-Regular.ttf").await?,
            egui: Default::default()
        })
    }

    fn load_egui(&mut self, ctx: &egui::Context) {
        self.egui.sprites.get_or_insert_with(|| Self::load_egui_texture(ctx, "sprites", self.sprites));
        self.egui.tileset.get_or_insert_with(|| Self::load_egui_texture(ctx, "tileset", self.tileset));
    }

    fn load_egui_texture(ctx: &egui::Context, name: impl ToString, texture: Texture2D) -> egui::TextureHandle {
        let image = texture.get_texture_data();
        let size = [image.width(), image.height()];
        let egui_image = egui::ColorImage::from_rgba_unmultiplied(size, &image.bytes);
        ctx.load_texture(name.to_string(), egui_image)
    }
}

pub async fn game_screen(network: Networking) {
    let mut assets = Assets::load().await
        .expect("Could not load assets");

    let mut state = GameState::with_network(network);
    let mut hovering_egui = false;

    egui_macroquad::cfg(|ctx| assets.load_egui(ctx));

    loop {
        let time = get_time();

        // update
        state.update_network(time);
        egui_macroquad::ui(|ctx| {
            state.update_ui(ctx, &assets);
            hovering_egui = ctx.wants_pointer_input();
        });
        
        for player in state.players.iter_mut() {
            player.update(time);
        }

        if state.my_id.is_some() {
            if state.movement_lock <= time {
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
                    let new_position = state.me().unwrap().position + direction.offset().into();
                    if state.map.valid(new_position) {
                        let me = state.me_mut().unwrap();
                        let from = me.position;
                        me.position = new_position;
                        me.direction = direction;

                        me.set_tween(from, time, MOVEMENT_SPEED);

                        state.network.send(ClientMessage::Move(direction));
                        state.movement_lock = time + MOVEMENT_SPEED;
                    }
                }
            }
        }
        
        let mouse_button = if hovering_egui {
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
            let (mx, my) = mouse_position();
            let tile_pos = ivec2(mx as i32, my as i32) / 48;

            if state.last_tile != Some(tile_pos) {
                let tile = state.map.tile_mut(state.layer, tile_pos).unwrap();
                *tile = match mouse_button {
                    MouseButton::Left if state.is_autotile => Tile::autotile(state.coords),
                    MouseButton::Left => Tile::basic(state.coords),
                    MouseButton::Right => Tile::empty(),
                    _ => unreachable!()
                };

                let coords = match mouse_button {
                    MouseButton::Left => Some(state.coords.into()),
                    MouseButton::Right => None,
                    _ => unreachable!()
                };

                state.map.update_autotiles();

                state.network.send(ClientMessage::ChangeTile(
                    tile_pos.into(),
                    state.layer,
                    coords,
                    state.is_autotile
                ));

                state.last_tile = Some(tile_pos);
            }
        }

        // draw
        clear_background(BLACK);

        state.map.draw(MapLayer::Ground, &assets);
        state.map.draw(MapLayer::Mask, &assets);

        for player in &state.players {
            player.draw(time, &assets);
        }

        state.map.draw(MapLayer::Fringe, &assets);

        egui_macroquad::draw();

        next_frame().await;
    }
}
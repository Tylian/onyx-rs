use std::collections::HashMap;

use onyx_common::{SPRITE_SIZE, TILE_SIZE, WALK_SPEED, RUN_SPEED};
use onyx_common::network::{ClientId, ChatMessage, ServerMessage, ClientMessage, Direction, MapLayer, AreaData, TileAnimation};
use macroquad::prelude::*;
use log::{error, info, debug};

use crate::assets::Assets;
use crate::networking::{NetworkClient, NetworkStatus};
use crate::map::{Map, Tile, Area, draw_area};
use crate::ui::{area_radio, tile_selector};
use self::player::{Player, Animation, Tween};

mod player;

#[derive(Eq, PartialEq)]
enum MapEditor {
    Closed,
    Tileset,
    Areas,
    Settings,
}

struct UiState {
    chat_message: String,
    chat_messages: Vec<ChatMessage>,
    layer: MapLayer,
    tile_pos: egui::Pos2,
    is_autotile: bool,
    last_tile: Option<(MouseButton, IVec2)>,
    drag_start: Option<Vec2>,
    map_editor: MapEditor,
    block_pointer: bool,
    block_keyboard: bool,
    map_width: u32,
    map_height: u32,
    area: AreaData,
    animated: bool,
    tile_animation: TileAnimation,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            chat_message: String::new(),
            chat_messages: Vec::new(),
            layer: MapLayer::Ground,
            tile_pos: egui::Pos2::default(),
            is_autotile: false,
            last_tile: None,
            map_editor: MapEditor::Closed,
            block_pointer: false,
            block_keyboard: false,
            map_width: 0,
            map_height: 0,
            drag_start: Option::default(),
            area: AreaData::Blocked,
            animated: false,
            tile_animation: TileAnimation {
                frames: 2,
                duration: 1.0,
                bouncy: false
            }
        }
    }
}

impl UiState {
    /// Get the currently selected properties in the map editor as a [Tile] instance
    fn get_tile(&self) -> Tile {
        Tile {
            texture: ivec2(self.tile_pos.x as i32 / TILE_SIZE, self.tile_pos.y as i32 / TILE_SIZE),
            autotile: self.is_autotile,
            animation: if self.animated {
                Some(self.tile_animation)
            } else {
                None
            },
        }
    }
}

struct GameState {
    assets: Assets,
    network: NetworkClient,
    players: HashMap<ClientId, Player>,
    local_player: Option<ClientId>,
    map: Map,
    ui_state: UiState,
    start_time: f64,
    time: f64,
    camera: Camera2D,
    last_movement: Option<(Direction, f64)>,
}

impl GameState {
    fn new(network: NetworkClient, assets: Assets) -> Self {
        Self {
            assets,
            network,
            players: Default::default(),
            local_player: Default::default(),
            map: Map::new(20, 15),
            ui_state: Default::default(),
            last_movement: Default::default(),
            start_time: get_time(),
            time: get_time(),
            camera: Camera2D::default()
        }
    }

    #[allow(dead_code)]
    fn elapsed(&self) -> f64 {
        self.time - self.start_time
    }

    fn process_message(&mut self, text: String) {
        if text.starts_with("/mapeditor") {
            self.ui_state.map_editor = MapEditor::Tileset;
        } else {
            self.network.send(ClientMessage::Message(text));
        }
    }

    fn update(&mut self) {
        self.time = get_time();

        self.update_network();
        egui_macroquad::ui(|ctx| {
            self.update_ui(ctx);
            self.ui_state.block_pointer = ctx.wants_pointer_input();
            self.ui_state.block_keyboard = ctx.wants_keyboard_input();
        });
        
        self.update_players();

        self.update_input();
        self.update_camera();
    }

    fn update_players(&mut self) {
        for player in self.players.values_mut() {
            if let Some(tween) = player.tween.as_mut() {
                let offset = tween.velocity * (self.time - tween.last_update) as f32;
                let new_position = player.position + offset;
                // only block on the bottom half of the sprite, feels better
                let sprite_rect = Rect::new(new_position.x, new_position.y + SPRITE_SIZE as f32 / 2.0, SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0);
                let (map_width, map_height) = self.map.pixel_size();

                let valid = sprite_rect.left() >= 0.0 && sprite_rect.top() >= 0.0
                    && sprite_rect.right() < map_width && sprite_rect.bottom() < map_height
                    && !self.map.areas.iter().any(|attrib| attrib.data == AreaData::Blocked && attrib.position.overlaps(&sprite_rect));

                if valid {
                    player.position = new_position;
                }

                // ? need to update anyway even if we don't change anything
                // ? if we don't you can clip through stuff by walking against it for awhile
                tween.last_update = self.time;
            }
        }
    }

    fn update_network(&mut self) {
        if self.network.status() != NetworkStatus::Connected {
            return;
        }

        let time = self.time;
        while let Some(message) = self.network.try_recv() {
            debug!("{:?}", message);
            match message {
                ServerMessage::Hello(id) => {
                    self.local_player = Some(id);
                },
                ServerMessage::PlayerJoined(id, player_data) => {
                    self.players.insert(id, Player::from_network(id, player_data));
                },
                ServerMessage::PlayerLeft(id) => {
                    // self.players.retain(|p| p.id != id);
                    self.players.remove(&id);
                },
                ServerMessage::Message(message) => {
                    self.ui_state.chat_messages.push(message);
                },
                ServerMessage::ChangeMap(remote) => {
                    match remote.try_into() {
                        Ok(map) => self.map = map,
                        Err(err) => error!("Error converting remote map: {err:?}"),
                    };
                },
                ServerMessage::PlayerMoved { client_id, position, direction, velocity } => {
                    if let Some(player) = self.players.get_mut(&client_id) {
                        player.position = position.into();
                        player.direction = direction;
                        if let Some(velocity) = velocity {
                            let velocity = Vec2::from(velocity);
                            player.animation = Animation::Walking { start: time, speed: velocity.length() as f64 };
                            player.tween = Some(Tween { velocity, last_update: time });
                        } else {
                            player.animation = Animation::Standing;
                            player.tween = None;
                        }
                    }
                },
            }
        }
    }

    fn chat_window(&mut self, ctx: &egui::Context) {
        use egui::*;
        use egui_extras::{StripBuilder, Size};

        let mut text: Option<Response> = None;
        let mut button: Option<Response> = None;

        let chat_window = Window::new("💬 Chat")
            .resizable(true)
            .default_pos(pos2(7., screen_height() - 198.)) // idfk lmao
            .default_size([367., 147.])
            .min_height(125.);

         // 7 522 386 708
         chat_window.show(ctx, |ui| {
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
                                            ui.colored_label(Color32::YELLOW, format!("[Server] {}", text));
                                        },
                                        ChatMessage::Say(text) => {
                                            ui.colored_label(Color32::WHITE, format!("[Say] {}", text));
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
    }

    fn map_editor(&mut self, ctx: &egui::Context) {
        use egui::*;
        
        Window::new("📝 Map Editor").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        let data = self.map.clone().into();
                        self.network.send(ClientMessage::SaveMap(data));
                        self.ui_state.map_editor = MapEditor::Closed;
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        self.network.send(ClientMessage::RequestMap);
                        self.ui_state.map_editor = MapEditor::Closed;
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.selectable_value(&mut self.ui_state.map_editor, MapEditor::Tileset, "Tileset");
                ui.selectable_value(&mut self.ui_state.map_editor, MapEditor::Areas, "Areas");
                let settings = ui.selectable_value(&mut self.ui_state.map_editor, MapEditor::Settings, "Settings");

                if settings.clicked() {
                    self.ui_state.map_width = self.map.width;
                    self.ui_state.map_height = self.map.height;
                }
            });
            ui.separator();

            match self.ui_state.map_editor {
                MapEditor::Tileset => {
                    let id = ui.make_persistent_id("mapeditor_settings");
                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Layer: ");
                                egui::ComboBox::from_id_source("layer")
                                    .selected_text(self.ui_state.layer.to_string())
                                    .show_ui(ui, |ui| {
                                        for layer in MapLayer::iter() {
                                            if layer == MapLayer::Fringe {
                                                ui.separator();
                                            }
                                            ui.selectable_value(&mut self.ui_state.layer, layer, layer.to_string());
                                        }
                                    }
                                );
                                ui.weak("(Press the arrow for more options)");
                            });
                        }).body(|ui| {
                            ui.checkbox(&mut self.ui_state.is_autotile, "Autotile");
                            ui.checkbox(&mut self.ui_state.animated, "Animated");
                            ui.add_enabled_ui(self.ui_state.animated, |ui| {
                                Grid::new("animation settings").num_columns(2).show(ui, |ui| {
                                    ui.label("Duration:");
                                    ui.add(DragValue::new(&mut self.ui_state.tile_animation.duration).speed(0.01f64).clamp_range(0f64..=f64::MAX).suffix("s"));
                                    ui.end_row();

                                    ui.label("Frames:");
                                    ui.add(DragValue::new(&mut self.ui_state.tile_animation.frames).speed(0.1f64).clamp_range(0f64..=f64::MAX));
                                    ui.end_row();
                                });
                                ui.checkbox(&mut self.ui_state.tile_animation.bouncy, "Bouncy animation (e.g 1-2-3-2)");
                            });
                        });
                        
                    if let Some(texture) = self.assets.egui.tileset.as_ref() {
                        tile_selector(ui, texture, &mut self.ui_state.tile_pos, Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32));
                    };
                }
                MapEditor::Areas => { 
                    ui.horizontal(|ui| {
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.heading("Area type");
                                let response = area_radio(ui, matches!(self.ui_state.area, AreaData::Blocked),
                                    "Blocked", "Entities are blocked from entering this area.");
                                if response.clicked() {
                                    self.ui_state.area = AreaData::Blocked;
                                }

                                let response = area_radio(ui, matches!(self.ui_state.area, AreaData::Log(_)),
                                    "Log", "Debug area, sends a message when inside.");
                                if response.clicked() {
                                    self.ui_state.area = AreaData::Log(Default::default());
                                }

                            });
                        });

                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.heading("Area data");
                                Grid::new("area data").num_columns(2).show(ui, |ui| {
                                    match &mut self.ui_state.area {
                                        AreaData::Blocked => { ui.label("Blocked has no values"); },
                                        AreaData::Log(message) => {
                                            ui.label("Greeting:");
                                            ui.text_edit_singleline(message);
                                            ui.end_row();
                                        },
                                    }
                                });
                            });
                        });
                    });
                    
                },
                MapEditor::Settings => {
                    ui.group(|ui| {
                        ui.heading("Map size");
                        Grid::new("resize").num_columns(2).show(ui, |ui| {
                            ui.label("Width:");
                            ui.add(DragValue::new(&mut self.ui_state.map_width).clamp_range(0..=u32::MAX).speed(0.05).suffix(" tiles"));
                            ui.end_row();

                            ui.label("Height:");
                            ui.add(DragValue::new(&mut self.ui_state.map_height).clamp_range(0..=u32::MAX).speed(0.05).suffix(" tiles"));
                            ui.end_row();

                            ui.add_enabled_ui(is_key_down(KeyCode::LeftShift), |ui| {
                                let button = ui.button("Save").on_disabled_hover_ui(|ui| {
                                    ui.colored_label(Color32::RED, "This will destroy tiles outside of the map and isn't reversable.");
                                    ui.label("Hold shift to enable the save button.");
                                });
                                if button.clicked() {
                                    self.map = self.map.resize(self.ui_state.map_width, self.ui_state.map_height);
                                }
                            });
                        });
                    });  
                },

                // specifically needs to be empty cause for 1 frame after closing it this is shown lol
                MapEditor::Closed => (),
            }
        });
    }

    fn update_ui(&mut self, ctx: &egui::Context) {
        self.chat_window(ctx);

        if self.ui_state.map_editor != MapEditor::Closed {
            self.map_editor(ctx);
        }

        /*egui::Window::new("📝 Memory")
        .resizable(false)
        .show(&egui_ctx, |ui| {
            egui_ctx.memory_ui(ui);
        });*/
    }

    fn update_input(&mut self) {
        if !self.ui_state.block_keyboard {
            if let Some(player) = self.local_player.and_then(|id| self.players.get_mut(&id)) {
                let movement = if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                    Some(Direction::North)
                } else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S)  {
                    Some(Direction::South)
                } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A)  {
                    Some(Direction::West)
                } else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D)  {
                    Some(Direction::East)
                } else {
                    None
                };
    
                let speed = if is_key_down(KeyCode::LeftShift) { WALK_SPEED } else { RUN_SPEED };
                let cache = movement.zip(Some(speed)); // lol
    
                if cache != self.last_movement {
                    self.last_movement = cache;
                    let velocity = if let Some(direction) = movement {
                        let velocity = Vec2::from(direction.offset_f32()) * speed as f32;
    
                        player.animation = Animation::Walking { start: self.time, speed };
                        player.tween = Some(Tween { velocity, last_update: self.time });
                        player.direction = direction;
    
                        Some(velocity.into())
                        
                    } else {
                        player.animation = Animation::Standing;
                        player.tween = None;
                        None
                    };

                    self.network.send(ClientMessage::Move { position: player.position.into(), direction: player.direction, velocity });
                }
            }
    
            // Admin
            if is_key_pressed(KeyCode::F1) {
                self.ui_state.map_editor = MapEditor::Tileset;
            }
        }

        if !self.ui_state.block_pointer {
            // Map editor
            match self.ui_state.map_editor {
                MapEditor::Tileset => {
                    let mouse_button = if is_mouse_button_down(MouseButton::Left) {
                        Some(MouseButton::Left)
                    } else if is_mouse_button_down(MouseButton::Right) {
                        Some(MouseButton::Right)
                    } else {
                        None
                    };
            
                    if let Some(mouse_button) = mouse_button {
                        let mouse_position = self.camera.screen_to_world(mouse_position().into()).as_i32();
                        let tile_position = mouse_position / TILE_SIZE;
            
                        let current_tile = (mouse_button, tile_position);
            
                        if self.ui_state.last_tile != Some(current_tile) {
                            match mouse_button {
                                MouseButton::Left => {
                                    let tile = self.ui_state.get_tile();
                                    self.map.set_tile(self.ui_state.layer, tile_position, tile);
                                },
                                MouseButton::Right => {
                                    self.map.clear_tile(self.ui_state.layer, tile_position);
                                },
                                _ => (),
                            };

                            self.map.update_autotile_cache();
                            self.ui_state.last_tile = Some(current_tile);
                        }
                    }
                },
                MapEditor::Areas => {
                    let mouse_position = self.camera.screen_to_world(mouse_position().into());
                    if is_mouse_button_pressed(MouseButton::Right) {
                        for (i, attrib) in self.map.areas.iter().enumerate().rev() {
                            if attrib.position.contains(mouse_position) {
                                self.map.areas.swap_remove(i);
                                break;
                            }
                        }
                    }
                    
                    let mouse_down = is_mouse_button_down(MouseButton::Left);
                    if self.ui_state.drag_start.is_some() && !mouse_down {
                        let drag_start = self.ui_state.drag_start.take().unwrap();
                        let start = drag_start.min(mouse_position);
                        let size = (drag_start.max(mouse_position) - start)
                            .max(vec2(6.0, 6.0)); // assume that 6x6 is the smallest you can make.

                        let drag_rect = Rect::new(start.x, start.y, size.x, size.y);

                        self.map.areas.push(Area {
                            position: drag_rect,
                            data: self.ui_state.area.clone(),
                        });
                    } else if self.ui_state.drag_start.is_none() && mouse_down {
                        self.ui_state.drag_start = Some(mouse_position);
                    };
                },
                _ => (),
            }
        }
    }

    fn update_camera(&mut self) {
        if let Some(player) = self.local_player.and_then(|id| self.players.get_mut(&id)) {
            let min = Vec2::ZERO;
            let max = vec2(
                self.map.width as f32 * TILE_SIZE as f32 - screen_width(),
                self.map.height as f32 * TILE_SIZE as f32 - screen_height()
            );
            
            let mut position = -vec2(screen_width() / 2., screen_height() / 2.);
            position += player.position + vec2(24., 24.);
            position = position.clamp(min, max);

            let (map_width, map_height) = self.map.pixel_size();

            // if the map is too small, center it
            if map_width <= screen_width() {
                position.x = (map_width - screen_width()) / 2.;
            }

            if map_height <= screen_height() {
                position.y = (map_height - screen_height()) / 2.;
            }

            let rect = Rect::new(position.x, position.y, screen_width(), screen_height());
            self.camera = Camera2D::from_display_rect(rect);
        }
    }

    fn draw(&mut self) {
        clear_background(BLACK);

        let (map_width, map_height) = self.map.pixel_size();
        draw_rectangle_lines(-3., -3., map_width + 6., map_height + 6., 6., GRAY);

        self.map.draw_layer(MapLayer::Ground, self.time, &self.assets);
        self.map.draw_layer(MapLayer::Mask, self.time, &self.assets);
        self.map.draw_layer(MapLayer::Mask2, self.time, &self.assets);

        for player in self.players.values() {
            player.draw(self.time, &self.assets);
        }

        self.map.draw_layer(MapLayer::Fringe, self.time, &self.assets);
        self.map.draw_layer(MapLayer::Fringe2, self.time, &self.assets);

        if self.ui_state.map_editor != MapEditor::Closed {
            self.map.draw_areas(&self.assets);
            if let Some(drag_start) = self.ui_state.drag_start {
                let mouse = self.camera.screen_to_world(mouse_position().into());
                let start = drag_start.min(mouse);
                let end = drag_start.max(mouse);
                let size = end - start;

                let drag_rect = Rect::new(start.x, start.y, size.x, size.y);
                draw_area(drag_rect, &self.ui_state.area, &self.assets);
            }
        }
    }
}

pub async fn game_screen(network: NetworkClient, assets: Assets) {
    let mut state = GameState::new(network, assets);

    loop {
        state.update();

        set_camera(&state.camera);
        state.draw();
        set_default_camera();

        egui_macroquad::draw();

        next_frame().await;
    }
}
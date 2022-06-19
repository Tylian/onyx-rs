use std::collections::HashMap;

use onyx_common::network::{ClientId, ChatMessage, ServerMessage, ClientMessage, Direction, MapLayer, AttributeData};
use macroquad::prelude::*;
use log::{error, info, debug};

use crate::assets::Assets;
use crate::networking::{NetworkClient, NetworkStatus};
use crate::map::{Map, Tile, Attribute};
use self::player::{Player, Animation, Tween};

mod player;

pub const TILE_SIZE: f32 = 48.;
pub const TILE_SIZE_I: i32 = 48;
pub const SPRITE_SIZE: f32 = 48.;
pub const SPRITE_SIZE_I: f32 = 48.;

pub const WALK_SPEED: f64 = 2.5 * TILE_SIZE as f64;
pub const RUN_SPEED: f64 = 5.0 * TILE_SIZE as f64;

fn ivec2_to_egui(ivec: IVec2) -> egui::Vec2 {
    egui::Vec2::new(ivec.x as f32, ivec.y as f32)
}

fn egui_to_ivec2(pos: egui::Pos2) -> IVec2 {
    ivec2(pos.x as i32, pos.y as i32)
}

#[derive(Eq, PartialEq)]
enum MapEditor {
    Closed,
    Tileset,
    Attributes,
    Settings,
}

struct UiState {
    chat_message: String,
    chat_messages: Vec<ChatMessage>,
    layer: MapLayer,
    coords: IVec2,
    is_autotile: bool,
    last_tile: Option<(MouseButton, IVec2)>,
    drag_start: Option<Vec2>,
    map_editor: MapEditor,
    block_pointer: bool,
    block_keyboard: bool,
    map_width: u32,
    map_height: u32,
    attribute: AttributeData,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            chat_message: String::new(),
            chat_messages: Vec::new(),
            layer: MapLayer::Ground,
            coords: IVec2::default(),
            is_autotile: false,
            last_tile: None,
            map_editor: MapEditor::Closed,
            block_pointer: false,
            block_keyboard: false,
            map_width: 0,
            map_height: 0,
            drag_start: Option::default(),
            attribute: AttributeData::Blocked,
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
                let offset = tween.speed * (self.time - tween.last_update) as f32;
                let new_position = player.position + offset;
                // only block on the bottom half of the sprite, feels better
                let sprite_rect = Rect::new(new_position.x, new_position.y + SPRITE_SIZE / 2.0, SPRITE_SIZE, SPRITE_SIZE / 2.0);
                let (map_width, map_height) = self.map.pixel_size();

                let valid = sprite_rect.left() >= 0.0 && sprite_rect.top() >= 0.0
                    && sprite_rect.right() < map_width && sprite_rect.bottom() < map_height
                    && !self.map.attributes.iter().any(|attrib| attrib.data == AttributeData::Blocked && attrib.position.overlaps(&sprite_rect));

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
                ServerMessage::ChangeTile { position, layer, tile: uv, is_autotile }  => {
                    if let Some(tile) = self.map.tile_mut(layer, position.into()) {
                        *tile = match uv {
                            Some(uv) if is_autotile => Tile::autotile(uv.into()),
                            Some(uv) => Tile::basic(uv.into()),
                            None => Tile::Empty
                        };
                        self.map.update_autotiles();
                    }
                },
                ServerMessage::ChangeMap(remote) => {
                    match remote.try_into() {
                        Ok(map) => self.map = map,
                        Err(err) => error!("Error converting remote map: {err:?}"),
                    };
                },
                ServerMessage::PlayerMoved { client_id, position, direction, velocity } => {
                    if let Some(player) = self.players.get_mut(&client_id) {
                        let speed = Vec2::from(velocity).length();
                        // todo keep starting time?
                        player.position = position.into();
                        player.animation = Animation::Walking { start: time, speed: speed as f64 };
                        player.tween = Some(Tween { speed: velocity.into(), last_update: time });
                        player.direction = direction;
                    }
                },
                ServerMessage::PlayerStopped { client_id, position, direction } => {
                    if let Some(player) = self.players.get_mut(&client_id) {
                        player.position = position.into();
                        player.direction = direction;
                        player.animation = Animation::Standing;
                        player.tween = None;
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
                ui.selectable_value(&mut self.ui_state.map_editor, MapEditor::Attributes, "Attributes");
                let settings = ui.selectable_value(&mut self.ui_state.map_editor, MapEditor::Settings, "Settings");

                if settings.clicked() {
                    self.ui_state.map_width = self.map.width;
                    self.ui_state.map_height = self.map.height;
                }
            });
            ui.separator();

            match self.ui_state.map_editor {
                MapEditor::Tileset => {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_label("Layer")
                            .selected_text(format!("{:?}", self.ui_state.layer)) // todo: don't use debug
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.ui_state.layer, MapLayer::Ground, "Ground");
                                ui.selectable_value(&mut self.ui_state.layer, MapLayer::Mask, "Mask");
                                ui.separator();
                                ui.selectable_value(&mut self.ui_state.layer, MapLayer::Fringe, "Fringe");
                            }
                        );
                        // ui.radio_value(&mut self.ui_state.layer, MapLayer::Ground, "Ground");
                        // ui.radio_value(&mut self.ui_state.layer, MapLayer::Mask, "Mask");
                        // ui.radio_value(&mut self.ui_state.layer, MapLayer::Fringe, "Fringe").on_hover_text("Displayed above players & NPCs");
                        ui.separator();
                        ui.checkbox(&mut self.ui_state.is_autotile, "Is autotile?");
                    });

                    if let Some(texture) = self.assets.egui.tileset.as_ref() {
                        ScrollArea::both().show_viewport(ui, |ui, viewport| {
                            let image = Image::new(texture, texture.size_vec2())
                                .sense(Sense::click());
        
                            let clip_rect = ui.clip_rect();
        
                            let response = ui.add(image);
                            if response.clicked() {
                                let pos = response.interact_pointer_pos().expect("Pointer position shouldn't be None");
                                let offset = viewport.left_top() + (pos - clip_rect.left_top()); // weird order just to make it typecheck lol
                                self.ui_state.coords = egui_to_ivec2(offset) / TILE_SIZE_I;
                            }
        
                            let pos = (clip_rect.left_top() - viewport.left_top()) + ivec2_to_egui(self.ui_state.coords * TILE_SIZE_I);
                            let rect = Rect::from_min_size(pos.to_pos2(), Vec2::new(TILE_SIZE, TILE_SIZE));
        
                            // todo: this is offset slightly by the stroke?
                            let painter = ui.painter();
                            painter.rect_stroke(rect, 0., ui.visuals().window_stroke());
        
                            response
                        });
                    }
                }
                MapEditor::Attributes => { 
                    ui.horizontal(|ui| {
                        ui.group(|ui| {
                            ui.radio_value(&mut self.ui_state.attribute, AttributeData::Blocked, "Blocked");
                        });

                        ui.group(|ui| {
                            match self.ui_state.attribute {
                                AttributeData::Blocked => ui.label("Blocked has no values"),
                            }
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
                    if let Some(direction) = movement {
                        let velocity = Vec2::from(direction.offset_f32()) * speed as f32;
    
                        // todo keep starting time?
                        player.animation = Animation::Walking { start: self.time, speed };
                        player.tween = Some(Tween { speed: velocity, last_update: self.time });
                        player.direction = direction;
    
                        self.network.send(ClientMessage::Move { position: player.position.into(), direction, velocity: velocity.into() });
                    } else {
                        player.animation = Animation::Standing;
                        player.tween = None;
    
                        self.network.send(ClientMessage::StopMoving { position: player.position.into(), direction: player.direction });
                    }
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
                        let tile_position = mouse_position / TILE_SIZE_I;
            
                        let current_tile = (mouse_button, tile_position);
            
                        if self.ui_state.last_tile != Some(current_tile) {
                            if let Some(tile) = self.map.tile_mut(self.ui_state.layer, tile_position) {
                                *tile = match mouse_button {
                                    MouseButton::Left if self.ui_state.is_autotile => Tile::autotile(self.ui_state.coords),
                                    MouseButton::Left => Tile::basic(self.ui_state.coords),
                                    MouseButton::Right => Tile::empty(),
                                    _ => unreachable!()
                                };
                                self.map.update_autotiles();
                            }
                            
                            self.ui_state.last_tile = Some(current_tile);
                        }
                    }
                },
                MapEditor::Attributes => {
                    let mouse_position = self.camera.screen_to_world(mouse_position().into());
                    if is_mouse_button_pressed(MouseButton::Right) {
                        for (i, attrib) in self.map.attributes.iter().enumerate().rev() {
                            if attrib.position.contains(mouse_position) {
                                self.map.attributes.swap_remove(i);
                                break;
                            }
                        }
                    }
                    
                    let mouse_down = is_mouse_button_down(MouseButton::Left);
                    if self.ui_state.drag_start.is_some() && !mouse_down {
                        let drag_start = self.ui_state.drag_start.take().unwrap();
                        let start = drag_start.min(mouse_position);
                        let size = drag_start.max(mouse_position) - start;

                        let drag_rect = Rect::new(start.x, start.y, size.x, size.y);

                        self.map.attributes.push(Attribute {
                            position: drag_rect,
                            data: self.ui_state.attribute,
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
            let min = vec2(0., 0.);
            let max = vec2(self.map.width as f32 * TILE_SIZE - screen_width(), self.map.height as f32 * TILE_SIZE - screen_height());
            
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

        self.map.draw_layer(MapLayer::Ground, &self.assets);
        self.map.draw_layer(MapLayer::Mask, &self.assets);

        for player in self.players.values() {
            player.draw(self.time, &self.assets);
        }

        self.map.draw_layer(MapLayer::Fringe, &self.assets);

        if self.ui_state.map_editor != MapEditor::Closed {
            self.map.draw_attributes(&self.assets);
            if let Some(drag_start) = self.ui_state.drag_start {
                let mouse = self.camera.screen_to_world(mouse_position().into());
                let start = drag_start.min(mouse);
                let end = drag_start.max(mouse);
                let size = end - start;

                let drag_rect = Rect::new(start.x, start.y, size.x, size.y);
                let attrib = Attribute {
                    position: drag_rect,
                    data: self.ui_state.attribute,
                };

                attrib.draw(&self.assets);
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
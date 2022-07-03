use std::collections::HashMap;
use std::rc::Rc;

use common::{
    network::{ChatChannel, ClientId, client::Packet, server::Packet as ServerPacket, Direction, MapLayer, ZoneData},
    RUN_SPEED, SPRITE_SIZE, TILE_SIZE, WALK_SPEED,
};
use glam::{vec2, IVec2, Vec2};
use macroquad::{color, prelude::*};
use message_io::node::StoredNetEvent;

use crate::{
    assets::Assets,
    data::{draw_zone, Animation, Map, Player, Zone},
    network::Network,
    ui::{ChatWindow, MapEditor, Tab, Wants},
    utils::draw_text_shadow,
};

struct UiState {
    map_editor: MapEditor,
    map_editor_shown: bool,
    chat_window: ChatWindow,
    last_tile: Option<(MouseButton, IVec2)>,
    drag_start: Option<Vec2>,
    block_pointer: bool,
    block_keyboard: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            chat_window: ChatWindow::new(),
            last_tile: None,
            map_editor: MapEditor::new(),
            map_editor_shown: false,
            block_pointer: false,
            block_keyboard: false,
            drag_start: Option::default(),
        }
    }
}

struct State {
    assets: Rc<Assets>,
    network: Network,
    players: HashMap<ClientId, Player>,
    local_player: ClientId,
    map: Map,
    ui: UiState,
    start_time: f64,
    time: f64,
    clip_rect: Rect,
    camera: Camera2D,
    last_movement: Option<(Direction, f64)>,
}

impl State {
    fn new(network: Network, client_id: ClientId, assets: Rc<Assets>) -> Self {
        Self {
            assets,
            network,
            players: HashMap::default(),
            local_player: client_id,
            map: Map::new("start", 20, 15),
            ui: UiState::default(),
            last_movement: None,
            start_time: get_time(),
            time: get_time(),
            clip_rect: Rect::new(0.0, 0.0, screen_width(), screen_height()),
            camera: Camera2D::default(),
        }
    }

    #[allow(dead_code)]
    fn elapsed(&self) -> f64 {
        self.time - self.start_time
    }

    fn process_message(&mut self, channel: ChatChannel, text: String) {
        if text.starts_with("/pos") {
            let position = self.players[&self.local_player].position;
            let message = format!("Your position is x: {} y: {}", position.x, position.y);
            self.ui.chat_window.insert(ChatChannel::Echo, message);
        } else {
            self.network.send(&Packet::ChatMessage(channel, text));
        }
    }

    fn update(&mut self) {
        self.time = get_time();

        self.update_network();
        egui_macroquad::ui(|ctx| {
            self.update_ui(ctx);
            self.ui.block_pointer = ctx.wants_pointer_input();
            self.ui.block_keyboard = ctx.wants_keyboard_input();
        });

        self.update_players();

        self.update_input();
        self.update_camera();
    }

    fn update_players(&mut self) {
        let player_boxes = self
            .players
            .iter()
            .map(|(cid, player)| {
                (
                    *cid,
                    Rect::new(
                        player.position.x,
                        player.position.y + SPRITE_SIZE as f32 / 2.0,
                        SPRITE_SIZE as f32,
                        SPRITE_SIZE as f32 / 2.0,
                    ),
                )
            })
            .collect::<Vec<_>>();

        for (client_id, player) in &mut self.players {
            if let Some(&mut velocity) = player.velocity.as_mut() {
                let offset = velocity * (self.time - player.last_update) as f32;
                let new_position = player.position + offset;

                // only block on the bottom half of the sprite, feels better
                let sprite_rect = Rect::new(
                    new_position.x,
                    new_position.y + SPRITE_SIZE as f32 / 2.0,
                    SPRITE_SIZE as f32,
                    SPRITE_SIZE as f32 / 2.0,
                );
                let (map_width, map_height) = self.map.pixel_size();

                let mut valid = sprite_rect.left() >= 0.0
                    && sprite_rect.top() >= 0.0
                    && sprite_rect.right() < map_width
                    && sprite_rect.bottom() < map_height;

                if !player.flags.in_map_editor {
                    valid &= !player_boxes
                        .iter()
                        .filter(|(cid, _b)| cid != client_id)
                        .any(|(_, b)| b.overlaps(&sprite_rect));

                    valid &= !self
                        .map
                        .zones
                        .iter()
                        .filter(|zone| zone.data == ZoneData::Blocked)
                        .any(|zone| zone.position.overlaps(&sprite_rect));
                }

                if valid {
                    player.position = new_position;
                }

                // ? need to update anyway even if we don't change anything
                // ? if we don't you can clip through stuff by walking against it for awhile
                player.last_update = self.time;
            }
        }
    }

    fn change_map(&mut self, map: Map) {
        self.map = map;
        self.assets.toggle_music(self.map.settings.music.as_deref());
        self.assets.set_tileset(&self.map.settings.tileset).unwrap();
        if let Err(e) = self.map.save_cache() {
            log::error!("Couldn't save map cache: {}", e);
        }
    }

    fn update_ui(&mut self, ctx: &egui::Context) {
        use egui::Window;

        let mouse_position = self.camera.screen_to_world(mouse_position().into());

        // Show egui debugging
        #[cfg(debug_assertions)]
        if false {
            Window::new("🔧 Setting")
                .vscroll(true)
                .show(ctx, |ui| ctx.settings_ui(ui));
            Window::new("🔍 Inspection")
                .vscroll(true)
                .show(ctx, |ui| ctx.inspection_ui(ui));
            Window::new("🗺 Texture")
                .vscroll(true)
                .show(ctx, |ui| ctx.texture_ui(ui));
            Window::new("📝 Memory").vscroll(true).show(ctx, |ui| ctx.memory_ui(ui));
            Window::new("🖊 Style").vscroll(true).show(ctx, |ui| ctx.style_ui(ui));
        }

        self.ui.chat_window.show(ctx);
        if let Some((channel, message)) = self.ui.chat_window.message() {
            self.process_message(channel, message);
        }

        self.ui
            .map_editor
            .show(ctx, &self.assets, &mut self.ui.map_editor_shown);

        match self.ui.map_editor.wants() {
            None => (),
            Some(Wants::Save) => {
                let (id, settings) = self.ui.map_editor.map_settings();
                self.map.id = id.to_string();
                self.map.settings = settings.clone();

                let data = Box::new(self.map.clone().into());
                self.network.send(&Packet::SaveMap(data));
                self.network.send(&Packet::MapEditor(false));
            }
            Some(Wants::Close) => {
                self.network.send(&Packet::MapEditor(false));
                self.network.send(&Packet::RequestMap);
            }
            Some(Wants::Resize(width, height)) => {
                self.map.resize(width, height);
            }
            Some(Wants::Warp(id)) => {
                self.network.send(&Packet::Warp(id, None));
            }
            Some(Wants::Fill(layer, tile)) => {
                self.map.fill(layer, tile);
            }
        }

        if self.ui.map_editor_shown {
            for zone in &self.map.zones {
                if zone.position.contains(mouse_position) {
                    if let ZoneData::Warp(map_id, position, direction) = &zone.data {
                        egui::show_tooltip_at_pointer(ctx, egui::Id::new("zone_tooltip"), |ui| {
                            ui.label(format!("Warp to: {map_id}"));
                            ui.label(format!("x: {} y: {}", position.x, position.y));
                            if let Some(direction) = direction {
                                ui.label(format!("Stops movement, faces {}", direction));
                            } else {
                                ui.label("Keeps movement");
                            }
                        });
                    }
                }
            }
        }
    }

    fn update_input(&mut self) {
        if !self.ui.block_keyboard {
            self.update_keyboard();
        }
        if !self.ui.block_pointer { 
            self.update_pointer();
        }
    }

    fn update_keyboard(&mut self) {
        if let Some(player) = self.players.get_mut(&self.local_player) {
            let movement = if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                Some(Direction::North)
            } else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                Some(Direction::South)
            } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                Some(Direction::West)
            } else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                Some(Direction::East)
            } else {
                None
            };

            let speed = if is_key_down(KeyCode::LeftShift) {
                WALK_SPEED
            } else {
                RUN_SPEED
            };
            let cache = movement.zip(Some(speed)); // lol

            if cache != self.last_movement {
                self.last_movement = cache;
                let velocity = if let Some(direction) = movement {
                    let velocity = Vec2::from(direction.offset_f32()) * speed as f32;

                    player.animation = Animation::Walking {
                        start: self.time,
                        speed,
                    };
                    player.velocity = Some(velocity);
                    player.last_update = self.time;
                    player.direction = direction;

                    Some(velocity.into())
                } else {
                    player.animation = Animation::Standing;
                    player.velocity = None;
                    None
                };

                self.network.send(&Packet::Move {
                    position: player.position.into(),
                    direction: player.direction,
                    velocity,
                });
            }
        }

        // Admin
        if is_key_pressed(KeyCode::F1) {
            self.network.send(&Packet::MapEditor(true));
        }
    }

    fn update_pointer(&mut self) {

        // Map editor
        if self.ui.map_editor_shown {
            match self.ui.map_editor.tab() {
                Tab::Tileset => {
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

                        if self.ui.last_tile != Some(current_tile) {
                            match mouse_button {
                                MouseButton::Left => {
                                    let layer = self.ui.map_editor.layer();
                                    let tile = self.ui.map_editor.tile();
                                    self.map.set_tile(layer, tile_position, tile);
                                }
                                MouseButton::Right => {
                                    let layer = self.ui.map_editor.layer();
                                    self.map.clear_tile(layer, tile_position);
                                }
                                _ => (),
                            };

                            self.map.update_autotile_cache();
                            self.ui.last_tile = Some(current_tile);
                        }
                    }
                }
                Tab::Zones => {
                    let mouse_position = self.camera.screen_to_world(mouse_position().into());
                    if is_mouse_button_pressed(MouseButton::Right) {
                        for (i, attrib) in self.map.zones.iter().enumerate().rev() {
                            if attrib.position.contains(mouse_position) {
                                self.map.zones.swap_remove(i);
                                break;
                            }
                        }
                    }

                    let mouse_down = is_mouse_button_down(MouseButton::Left);
                    if self.ui.drag_start.is_some() && !mouse_down {
                        let drag_start = self.ui.drag_start.take().unwrap();
                        let start = drag_start.min(mouse_position);
                        let size = (drag_start.max(mouse_position) - start).max(vec2(6.0, 6.0)); // assume that 6x6 is the smallest you can make.

                        let drag_rect = Rect::new(start.x, start.y, size.x, size.y);

                        self.map.zones.push(Zone {
                            position: drag_rect,
                            data: self.ui.map_editor.zone_data().clone(),
                        });
                    } else if self.ui.drag_start.is_none() && mouse_down {
                        self.ui.drag_start = Some(mouse_position);
                    };
                }
                Tab::Settings | Tab::Tools => (),
            }
        }
    }

    fn update_camera(&mut self) {
        if let Some(player) = self.players.get_mut(&self.local_player) {
            let min = Vec2::ZERO;
            let max = vec2(
                self.map.width as f32 * TILE_SIZE as f32 - screen_width(),
                self.map.height as f32 * TILE_SIZE as f32 - screen_height(),
            );

            let mut position = -vec2(screen_width() / 2.0, screen_height() / 2.0);
            position += player.position + vec2(24.0, 24.0);
            position = position.clamp(min, max);

            let (map_width, map_height) = self.map.pixel_size();

            // if the map is too small, center it
            if map_width <= screen_width() {
                position.x = (map_width - screen_width()) / 2.0;
            }

            if map_height <= screen_height() {
                position.y = (map_height - screen_height()) / 2.0;
            }

            position.round();

            self.clip_rect = Rect::new(position.x, position.y, screen_width(), screen_height());
            self.camera = Camera2D::from_display_rect(self.clip_rect);
        }
    }

    fn draw(&self) {
        clear_background(color::BLACK);

        let (map_width, map_height) = self.map.pixel_size();
        draw_rectangle_lines(-3.0, -3.0, map_width + 6.0, map_height + 6.0, 6.0, color::GRAY);

        self.map.draw_layer(MapLayer::Ground, self.time, &self.assets);
        self.map.draw_layer(MapLayer::Mask, self.time, &self.assets);
        self.map.draw_layer(MapLayer::Mask2, self.time, &self.assets);

        let mut players = self.players.values().collect::<Vec<_>>();

        players.sort_by(|a, b| a.position.y.partial_cmp(&b.position.y).unwrap());

        for player in players {
            player.draw(self.time, &self.assets);
        }

        self.map.draw_layer(MapLayer::Fringe, self.time, &self.assets);
        self.map.draw_layer(MapLayer::Fringe2, self.time, &self.assets);

        if self.ui.map_editor_shown {
            self.map.draw_zones(&self.assets);
            if let Some(drag_start) = self.ui.drag_start {
                let mouse = self.camera.screen_to_world(mouse_position().into());
                let start = drag_start.min(mouse);
                let end = drag_start.max(mouse);
                let size = end - start;

                let drag_rect = Rect::new(start.x, start.y, size.x, size.y);
                draw_zone(drag_rect, self.ui.map_editor.zone_data(), &self.assets);
            }
        }

        draw_text_shadow(
            &self.map.settings.name,
            vec2(2.0, 2.0),
            TextParams {
                font: self.assets.font,
                color: color::WHITE,
                font_size: 20,
                ..Default::default()
            },
        );
    }
}

/// Networking
impl State {
    fn update_network(&mut self) {
        while let Some(event) = self.network.try_receive() {
            match event.network() {
                StoredNetEvent::Connected(_, _) => (),
                StoredNetEvent::Accepted(_, _) => unreachable!(),
                StoredNetEvent::Message(_, bytes) => {
                    let message = rmp_serde::from_slice(&bytes).unwrap();
                    self.handle_message(message);
                }
                StoredNetEvent::Disconnected(_) => todo!(),
            }
        }
    }

    fn handle_message(&mut self, message: ServerPacket) {
        match &message {
            ServerPacket::MapData(_) => log::debug!("MapData(..)"),
            message => {
                log::debug!("{message:?}");
            }
        }

        let time = self.time;
        match message {
            ServerPacket::JoinGame(_) | ServerPacket::FailedJoin(_) => unreachable!(),

            ServerPacket::PlayerJoined(id, player_data) => {
                self.players
                    .insert(id, Player::from_network(id, player_data, self.time));
            }
            ServerPacket::PlayerLeft(id) => {
                self.players.remove(&id);
            }
            ServerPacket::ChatLog(channel, message) => {
                self.ui.chat_window.insert(channel, message);
            }
            ServerPacket::ChangeMap(id, cache_id) => {
                self.players.clear();
                self.ui.map_editor_shown = false;

                let map = Map::from_cache(id);
                let needs_map = map
                    .as_ref()
                    .map(|map| map.settings.cache_key != cache_id)
                    .unwrap_or(true);

                if needs_map {
                    log::debug!("Requesting map..");
                    self.network.send(&Packet::RequestMap);
                } else {
                    log::debug!("Loading map from");
                    self.change_map(map.unwrap());
                }
            }
            ServerPacket::PlayerMove {
                client_id,
                position,
                direction,
                velocity,
            } => {
                if let Some(player) = self.players.get_mut(&client_id) {
                    player.position = position.into();
                    player.direction = direction;
                    if let Some(velocity) = velocity {
                        let velocity = Vec2::from(velocity);
                        player.animation = Animation::Walking {
                            start: time,
                            speed: velocity.length() as f64,
                        };
                        player.velocity = Some(velocity);
                        player.last_update = time;
                    } else {
                        player.animation = Animation::Standing;
                        player.velocity = None;
                    }
                }
            }
            ServerPacket::MapData(remote) => {
                let map = Map::try_from(*remote).unwrap();
                self.change_map(map);
            }
            ServerPacket::MapEditor {
                id,
                width,
                height,
                settings,
                maps,
            } => {
                self.ui.map_editor.update(
                    maps,
                    width,
                    height,
                    &*id,
                    settings,
                );
                self.ui.map_editor_shown = true;
            }
            ServerPacket::Flags(client_id, flags) => {
                self.players.get_mut(&client_id).unwrap().flags = flags;
            }
        }
    }
}

pub async fn run(network: Network, client_id: ClientId, assets: Rc<Assets>) {
    let mut state = State::new(network, client_id, assets);

    loop {
        state.update();

        set_camera(&state.camera);
        state.draw();
        set_default_camera();

        egui_macroquad::draw();

        draw_text_ex(
            &format!("{} fps", get_fps()),
            2.0,
            16.0,
            TextParams {
                font: state.assets.font,
                color: color::WHITE,
                ..Default::default()
            },
        );

        next_frame().await;
    }
}

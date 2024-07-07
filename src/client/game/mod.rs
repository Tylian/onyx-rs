use std::collections::HashMap;

use euclid::{approxeq::ApproxEq, size2};
use onyx::{ACCELERATION, FRICTION, RUN_SPEED, SPRITE_SIZE, TILE_SIZE, WALK_SPEED};
use onyx::math::units::{map, screen, world::{self, *}};
use onyx::network::{client::Packet, server::Packet as ServerPacket, ChatChannel, Direction, Entity, MapId, MapLayer, ZoneData};
use ggegui::egui::{Id, LayerId, Order};
use ggez::{
    event::MouseButton, input::keyboard::{KeyCode, KeyMods}, Context, GameResult
};
use renet::DefaultChannel;

use crate::{
    data::{draw_zone, Animation, Interpolation, Map, Player, Zone}, network::Network, scene::Transition, GameEvent, GameState
    // state::{UpdateContext, DrawContext, SetupContext, EventContext},
    // utils::{RectExt, rect, draw_text_shadow}
};

use self::{
    // camera::Camera,
    ui::{ChatWindow, MapEditor, Wants, Tab}
};

// mod camera;
mod ui;

pub struct GameScene {
    // assets: AssetCache,
    // camera: Camera,
    camera: Rect,
    last_position_send: f32,
    local_player: Entity,
    map: Map,
    network: Network,
    players: HashMap<Entity, Player>,
    ui: UiState,
}

impl GameScene {
    pub fn new(local_player: Entity, network: Network, ctx: &mut Context) -> Self {
        // let assets = AssetCache::load(ctx.assets, ctx.gfx).unwrap();
        //? technically incorrect, but correct in this case.
        let screen_size: Size2D = ctx.gfx.drawable_size().into();

        Self {
            // assets,
            local_player,
            players: HashMap::new(),
            network,
            map: Map::new(MapId::default(), size2(20, 15)),
            camera: Rect::new(Point2D::new(0.0, 0.0), screen_size),
            // camera: Camera::default(),
            ui: UiState::default(),
            last_position_send: 0.0,
        }
    }

    pub fn update(&mut self, ctx: &mut Context, state: &mut GameState) -> GameResult<Transition> {
        self.update_network(ctx);
        self.update_players(ctx);
        self.update_input(ctx);
        self.update_camera(ctx);

        self.update_gui(ctx, state);
        state.gui.update(ctx);

        Ok(Transition::None)
    }

    fn update_network(&mut self, ctx: &mut Context) {
        let delta = ctx.time.delta();

        self.network.client.update(delta);
        self.network.transport.update(delta, &mut self.network.client).unwrap();

        if let Some(e) = self.network.client.disconnect_reason() {
            panic!("Disconnected: {e}");
        }

        while let Some(bytes) = self.network.client.receive_message(DefaultChannel::ReliableUnordered) {
            match rmp_serde::from_slice(&bytes) {
                Ok(message) => self.handle_message(message, ctx),
                Err(e) => log::error!("Error parsing packet {:?}", e),
            };
        }

        const UPDATE_DELAY: f32 = 1.0 / 20.0; // update time in hz
        if ctx.time.time_since_start().as_secs_f32() >= self.last_position_send + UPDATE_DELAY {
            if let Some(player) = self.players.get(&self.local_player) {
                self.network.send(&Packet::Move {
                    position: player.position,
                    velocity: player.velocity,
                });
            }
        }

        if let Err(e) = self.network.transport.send_packets(&mut self.network.client) {
            panic!("Error sending packets: {e}");
        }
    }

    fn process_chat_message(&mut self, channel: ChatChannel, text: String) {
        if text.starts_with("/pos") {
            let position = self.players[&self.local_player].position;
            let message = format!("Your position is x: {} y: {}", position.x, position.y);
            self.ui.chat_window.insert(ChatChannel::Echo, message);
        } else {
            self.network.send(&Packet::ChatMessage(channel, text));
        }
    }

    fn handle_message(&mut self, message: ServerPacket, ctx: &mut Context) { 
        use ServerPacket::*;

        // Debug logging of server packets
        match &message {
            MapData(_) => log::debug!("MapData(..)"),
            message => {
                log::debug!("{message:?}");
            }
        }

        let time = ctx.time.time_since_start();
        
        match message {
            JoinGame(_) | FailedJoin(_) => unreachable!(),

            PlayerData(id, player_data) => {
                self.players.insert(id, Player::from_network(id, player_data, time.as_secs_f32()));
            }
            RemoveData(id) => {
                self.players.remove(&id);
            }
            ChatLog(channel, message) => {
                self.ui.chat_window.insert(channel, message);
            }
            ChangeMap(id, cache_id) => {
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
            PlayerMove {
                entity,
                position,
                velocity,
            } => {
                if let Some(player) = self.players.get_mut(&entity) {
                    if let Some(direction) = Direction::from_velocity(velocity) {
                        player.direction = direction;
                    }
                    
                    if !velocity.approx_eq(&Vector2D::zero()) {
                        player.velocity = velocity;
                        
                        // ? currently the player position is extrapolated if we don't recieve updates quick enough
                        // ? the below interpolation is based on the client's current local position as the starting point
                        // ? which during lag could look pretty wild. evaluate if this is the best approach
                        // ? the effect is entirely visual, so pick what would look best.
                        let interpolation = Interpolation {
                            initial: player.position,
                            target: position,
                            start_time: time.as_secs_f32(),
                            duration: 1.0 / 20.0,
                        };
                        player.interpolation = Some(interpolation);
                    } else {
                        player.animation = Animation::Standing;
                        player.velocity = Vector2D::zero();
                    }
                }
            }
            MapData(remote) => {
                let map = Map::try_from(*remote).unwrap();
                self.change_map(map);
            }
            MapEditor {
                id,
                width,
                height,
                settings,
                maps,
            } => {
                self.ui.map_editor.update(maps, width, height, id, settings);
                self.ui.map_editor_shown = true;
            }
            Flags(entity, flags) => {
                self.players.get_mut(&entity).unwrap().flags = flags;
            }
        }
    }

    fn update_players(&mut self, ctx: &mut Context) {
        let player_boxes: Vec<_> = self.players.iter()
            .map(|(cid, player)| (*cid, Player::collision_box(player.position)))
            .collect();
    
        let time = ctx.time.time_since_start().as_secs_f32();
        let dt = ctx.time.delta().as_secs_f32();

        let entity = self.local_player;
        if let Some(player) = self.players.get_mut(&entity) {
            let velocity = (player.velocity + player.acceleration ).clamp_length(0.0, player.max_speed);
            let friction_force = velocity.try_normalize().unwrap_or_default() * player.test_friction * dt;

            player.velocity = if friction_force.square_length() <= velocity.square_length() {
                velocity - friction_force
            } else {
                Vector2D::zero()
            };

            if player.velocity.square_length() >= f32::EPSILON * f32::EPSILON {
                let offset = player.velocity * dt;
                let new_position = player.position + offset;

                let sprite_box = Player::collision_box(new_position);
                let map_box = Box2D::from_size(self.map.world_size());

                let mut valid = map_box.contains_box(&sprite_box);

                if !player.flags.in_map_editor {
                    valid &= !player_boxes.iter()
                        .filter(|(id, _b)| *id != entity)
                        .any(|(_, b)| b.intersects(&sprite_box));

                    valid &= !self.map.zones.iter()
                        .filter(|zone| zone.data == ZoneData::Blocked)
                        .any(|zone| zone.position.intersects(&sprite_box));
                }

                if valid {
                    player.position = new_position;
                }
            } else {
                player.velocity = Vector2D::zero();
            }
        }

        for (entity, player) in &mut self.players {
            if *entity == self.local_player { continue; }
            if let Some(interpolation) = &mut player.interpolation {
                let elapsed = time - interpolation.start_time;
                let progress = elapsed / interpolation.duration;

                if progress < 1.0 {
                    let new_position = interpolation.initial.lerp(interpolation.target, progress);
                    let velocity = (new_position - player.position).normalize();
                    let new_direction = Direction::from_velocity(velocity);

                    player.position = interpolation.initial.lerp(interpolation.target, progress);
                    if let Some(direction) = new_direction {
                        player.direction = direction;
                    }
                } else {
                    player.position = interpolation.target;
                    player.interpolation = None;
                }

                player.update_animation(time);
            }
        }
    }

    fn change_map(&mut self, map: Map) {
        self.map = map;
        //self.assets.toggle_music(self.map.settings.music.as_deref());
        //self.assets.set_tileset(&self.map.settings.tileset).unwrap();
        if let Err(e) = self.map.save_cache() {
            log::error!("Couldn't save map cache: {}", e);
        }
    }

    fn update_gui(&mut self, ctx: &mut Context, state: &mut GameState) {
        use ggegui::egui;

        let gui_ctx = state.gui.ctx();
        let screen_size = self.screen_size(ctx);
        let mouse_position = self.screen_to_world(ctx, self.mouse_screen(ctx));
        
        // // Show egui debugging
        // #[cfg(debug_assertions)]
        // if false {
        //     Window::new("🔧 Setting")
        //         .vscroll(true)
        //         .show(ctx, |ui| ctx.settings_ui(ui));
        //     Window::new("🔍 Inspection")
        //         .vscroll(true)
        //         .show(ctx, |ui| ctx.inspection_ui(ui));
        //     Window::new("🗺 Texture")
        //         .vscroll(true)
        //         .show(ctx, |ui| ctx.texture_ui(ui));
        //     Window::new("📝 Memory").vscroll(true).show(ctx, |ui| ctx.memory_ui(ui));
        //     Window::new("🖊 Style").vscroll(true).show(ctx, |ui| ctx.style_ui(ui));
        // }

        self.ui.chat_window.show(&gui_ctx, screen_size.height);
        if let Some((channel, message)) = self.ui.chat_window.message() {
            self.process_chat_message(channel, message);
        }

        self.ui
            .map_editor
            .show(&gui_ctx, &mut state.assets, &mut self.ui.map_editor_shown);

        match self.ui.map_editor.wants() {
            None => (),
            Some(Wants::Save) => {
                let (id, settings) = self.ui.map_editor.map_settings();
                self.map.id = id;
                self.map.settings = settings.clone();

                let data = Box::new(self.map.clone().into());
                self.network.send(&Packet::SaveMap(data));
                self.network.send(&Packet::MapEditor(false));
            }
            Some(Wants::Close) => {
                self.network.send(&Packet::MapEditor(false));
                self.network.send(&Packet::RequestMap);
            }
            Some(Wants::Resize(size)) => {
                self.map.resize(size);
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
                        let layer_id = LayerId::new(Order::Foreground, Id::new("tooltip"));
                        egui::show_tooltip_at_pointer(&gui_ctx, layer_id, egui::Id::new("zone_tooltip"), |ui| {
                            ui.label(format!("Warp to map #{}", map_id.0));
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

        self.ui.block_keyboard = gui_ctx.wants_keyboard_input();
        self.ui.block_pointer = gui_ctx.wants_pointer_input();
    }

    fn update_input(&mut self, ctx: &mut Context) {
        if !self.ui.block_keyboard {
            self.update_keyboard(ctx);
        }
        if !self.ui.block_pointer {
            self.update_pointer(ctx);
        }
    }

    fn update_keyboard(&mut self, ctx: &mut Context) {
        let time = ctx.time.time_since_start().as_secs_f32();
        let keyboard = &ctx.keyboard;

        if let Some(player) = self.players.get_mut(&self.local_player) {
            let mut direction = Vector2D::zero();
            if keyboard.is_key_pressed(KeyCode::Up) || keyboard.is_key_pressed(KeyCode::W) {
                direction.y = -1.0;
            } else if keyboard.is_key_pressed(KeyCode::Down) || keyboard.is_key_pressed(KeyCode::S) {
                direction.y = 1.0;
            } else {
                direction.y = 0.0;
            }

            if keyboard.is_key_pressed(KeyCode::Left) || keyboard.is_key_pressed(KeyCode::A) {
                direction.x = -1.0;
            } else if keyboard.is_key_pressed(KeyCode::Right) || keyboard.is_key_pressed(KeyCode::D) {
                direction.x = 1.0;
            } else {
                direction.x = 0.0;
            }

            player.acceleration = direction.try_normalize().unwrap_or(Vector2D::zero()) * player.test_acceleration;
            player.max_speed = if keyboard.active_mods().contains(KeyMods::SHIFT) {
                WALK_SPEED
            } else {
                RUN_SPEED
            };

            // todo
            player.direction = Direction::from_velocity(player.acceleration).unwrap_or(player.direction);
            player.update_animation(time);

            if keyboard.is_key_just_pressed(KeyCode::Y) {
                player.test_acceleration += 1.0;
            } else if keyboard.is_key_just_pressed(KeyCode::H) {
                player.test_acceleration -= 1.0;
            }

            if keyboard.is_key_just_pressed(KeyCode::U) {
                player.test_friction += 1.0;
            } else if keyboard.is_key_just_pressed(KeyCode::J) {
                player.test_friction -= 1.0;
            }

            if keyboard.is_key_just_pressed(KeyCode::I) {
                player.test_acceleration *= 2.0;
            } else if keyboard.is_key_just_pressed(KeyCode::K) {
                player.test_acceleration /= 2.0;
            }

            if keyboard.is_key_just_pressed(KeyCode::O) {
                player.test_friction *= 2.0;
            } else if keyboard.is_key_just_pressed(KeyCode::L) {
                player.test_friction /= 2.0;
            }

            // let movement = if keyboard.is_key_pressed(KeyCode::Up) || keyboard.is_key_pressed(KeyCode::W) {
            //     Some(Direction::North)
            // } else if keyboard.is_key_pressed(KeyCode::Down) || keyboard.is_key_pressed(KeyCode::S) {
            //     Some(Direction::South)
            // } else if keyboard.is_key_pressed(KeyCode::Left) || keyboard.is_key_pressed(KeyCode::A) {
            //     Some(Direction::West)
            // } else if keyboard.is_key_pressed(KeyCode::Right) || keyboard.is_key_pressed(KeyCode::D) {
            //     Some(Direction::East)
            // } else {
            //     None
            // };

            // let speed = if keyboard.active_mods().contains(KeyMods::SHIFT) {
            //     WALK_SPEED
            // } else {
            //     RUN_SPEED
            // };
            // let cache = movement.map(|movement| (movement, speed)); // lol

            // if cache != self.last_movement {
            //     self.last_movement = cache;
            //     if let Some(direction) = movement {
            //         let velocity = Vec2::from(direction.offset_f32()) * speed;

            //         player.animation = Animation::Walking {
            //             start: time,
            //             speed,
            //         };
            //         player.velocity = Some(velocity);
            //         player.last_update = time;
            //         player.direction = direction;
            //     } else {
            //         player.animation = Animation::Standing;
            //         player.velocity = None;
            //     };
            // }
        }

        // Admin
        if keyboard.is_key_just_pressed(KeyCode::F1) {
            self.network.send(&Packet::MapEditor(true));
        }
    }

    fn update_pointer(&mut self, ctx: &mut Context) {
        let mouse_position = self.screen_to_world(ctx, self.mouse_screen(ctx));
        let mouse_valid = Box2D::from_size(self.map.world_size()).contains(mouse_position);

        // Map editor
        if self.ui.map_editor_shown {
            match self.ui.map_editor.tab() {
                Tab::Tileset => {
                    let mouse_button = if ctx.mouse.button_pressed(MouseButton::Left) {
                        Some(MouseButton::Left)
                    } else if ctx.mouse.button_pressed(MouseButton::Right) {
                        Some(MouseButton::Right)
                    } else {
                        None
                    };

                    if let Some(mouse_button) = mouse_button {
                        if mouse_valid {
                            let tile_position = (mouse_position / TILE_SIZE).floor().to_u32().cast_unit();
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
                }
                Tab::Zones => {
                    if ctx.mouse.button_just_pressed(MouseButton::Right) {
                        for (i, attrib) in self.map.zones.iter().enumerate().rev() {
                            if attrib.position.contains(mouse_position) {
                                self.map.zones.swap_remove(i);
                                break;
                            }
                        }
                    }

                    let mouse_down = ctx.mouse.button_pressed(MouseButton::Left);
                    if let Some((drag_start, drag_size)) = self.ui.drag_zone.as_mut() {
                        if mouse_down {
                            let size = (*drag_start - mouse_position)
                                .max(Vector2D::splat(6.0)) // assume that 6x6 is the smallest you can make.
                                .to_size(); 

                            *drag_size = size;
                        } else {
                            self.map.zones.push(Zone {
                                position: self.ui.drag_box2d().unwrap(),
                                data: self.ui.map_editor.zone_data().clone(),
                            });
                        }
                    } else if mouse_down {
                        self.ui.drag_zone = Some((mouse_position, Size2D::splat(6.0)));
                    };
                }
                Tab::Settings | Tab::Tools => (),
            }
        }
    }

    // // TODO: re-enable camera snapping
    #[allow(unused_variables)]
    fn update_camera(&mut self, ctx: &mut Context) {
        //? World coords = screen cords while zoom = 1
        let screen_size = self.screen_size(ctx)
            .cast_unit();

        if let Some(player) = self.players.get_mut(&self.local_player) {
            let map_size = self.map.world_size();

            // camera snap assumes 0 zoom, maybe it should include zoom?
            // include zoom by multplying the screen size by the 1 / zoom

            // let min = vec2(screen_width / 2.0, screen_height / 2.0);
            // let max = vec2(map_width - screen_width / 2.0, map_height - screen_height / 2.0);

            let half_sprite = Vector2D::new(SPRITE_SIZE, SPRITE_SIZE) / 2.0;
            let mut origin = player.position + half_sprite / 2.0;
            // target = target.clamp(min, max);

            // if the map is too small, center it
            if map_size.width <= screen_size.width {
                origin.x = map_size.width / 2.0;
            }

            if map_size.height <= screen_size.height {
                origin.y = map_size.height / 2.0;
            }

            // self.camera.target = target;
            self.camera = Rect::new(
                origin - screen_size / 2.0,
                screen_size
            );
        }
    }

    pub fn draw(&mut self, ctx: &mut Context, state: &mut GameState) -> GameResult {
        use ggez::graphics::*;

        let screen_size = self.screen_size(ctx);
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);

        let camera_rect = ggez::graphics::Rect::new(self.camera.origin.x, self.camera.origin.y, self.camera.size.width, self.camera.size.height);
        canvas.set_screen_coordinates(camera_rect);

        // Render time
        let time = ctx.time.time_since_start().as_secs_f32();

        // Draw first half of map
        self.map.draw_layers(ctx, &mut canvas, &[MapLayer::Ground, MapLayer::Mask, MapLayer::Mask2], &mut state.assets, time);

        // Draw players, NPCs, objects, etc.
        let mut players = self.players.values().collect::<Vec<_>>();
        players.sort_by(|a, b| a.position.y.partial_cmp(&b.position.y).unwrap());

        for player in players {
            player.draw(&mut canvas, time, &mut state.assets);
        }

        // Draw 2nd half of map.
        self.map.draw_layers(ctx, &mut canvas, &[MapLayer::Fringe, MapLayer::Fringe2], &mut state.assets, time);

        if self.ui.map_editor_shown {
            self.map.draw_zones(ctx, &mut canvas)?;

            if let Some(drag_box) = self.ui.drag_box2d() {
                draw_zone(ctx, &mut canvas, drag_box, self.ui.map_editor.zone_data())?;
            }
        }

        // UI drawing starts here
        canvas.set_screen_coordinates(Rect::new(0.0, 0.0, screen_size.width, screen_size.height));
        canvas.draw(&state.gui, DrawParam::default());
        
        // FPS
        let fps = ctx.time.fps();
        let fps_display = Text::new(format!("FPS: {fps:.02}"));
        canvas.draw(
            &fps_display,
            DrawParam::from([0.0, 0.0]).color(Color::WHITE),
        );

        if let Some(player) = self.players.get_mut(&self.local_player) {
            let text = Text::new(format!(
                "Velocity: {:.02?}  FRICTION: {:0.2}\nAcceleration: {:.02?}  ACCELERATION: {:0.2}\nPosition: {:0.2?}  Max Speed: {:.02}",
                player.velocity, player.test_friction,
                player.acceleration, player.test_acceleration,
                player.position, player.max_speed
            ));
            canvas.draw(&text, DrawParam::from([0.0, 14.0]).color(Color::WHITE));
        }

        canvas.finish(ctx)
    }

    pub fn event(&mut self, _ctx: &mut ggez::Context, _state: &mut GameState, event: GameEvent) -> GameResult {
        if event == GameEvent::Quit {
            self.network.transport.disconnect()
        }
        Ok(())
    }

    fn screen_to_world(&self, ctx: &Context, point: screen::Point2D) -> world::Point2D {
        let screen_size = self.screen_size(ctx);
        let camera_position = self.camera.origin;
        let camera_size = self.camera.size;

        let transform = euclid::Transform2D::scale(camera_size.width / screen_size.width, camera_size.height / screen_size.height)
            .then_translate(camera_position.to_vector());

        transform.transform_point(point)

        // point / screen_size * camera_size + camera_position
    }

    fn world_to_screen(&self, ctx: &Context, point: world::Point2D) -> screen::Point2D {
        use euclid::Transform2D;
        
        let screen_size = self.screen_size(ctx);
        let camera_position = self.camera.origin;
        let camera_size = self.camera.size;

        let transform = Transform2D::translation(-camera_position.x, -camera_position.y)
            .then_scale(screen_size.width / camera_size.width, screen_size.height / camera_size.height);

        transform.transform_point(point)
    }

    pub fn screen_size(&self, ctx: &Context) -> screen::Size2D {
        screen::Size2D::from(ctx.gfx.drawable_size())
    }

    pub fn mouse_screen(&self, ctx: &Context) -> screen::Point2D {
        screen::Point2D::from(ctx.mouse.position())
    }

    // fn draw(&mut self, ctx: &mut DrawContext) {
    //     self.assets.tick(ctx.gfx);

    //     if !self.assets.is_loaded() {
    //         let mut draw = ctx.gfx.create_draw();
    //         draw.clear(Color::BLACK);

    //         draw.text(&self.assets.debug_font, "Loading...")
    //             .color(Color::WHITE);

    //         ctx.gfx.render(&draw);
    //         return;
    //     }

    //     let time = ctx.app.timer.time_since_init();
    //     let (screen_width, screen_height) = ctx.app.window().size();
    //     let mouse_position = ctx.app.mouse.position();

    //     let mut draw = ctx.gfx.create_draw();
    //     draw.clear(Color::BLACK);

    //     // Set up camera position
    //     draw.transform().push(self.camera.matrix());

    //     // Render map border
    //     let (map_width, map_height) = self.map.pixel_size();
    //     draw.rect((-3.0, -3.0), (map_width + 6.0, map_height + 6.0))
    //         .color(Color::GRAY)
    //         .stroke(6.0);

    //     self.map.draw_layers(&mut draw, &[MapLayer::Ground, MapLayer::Mask, MapLayer::Mask2], &mut self.assets, time);

    //     let mut players = self.players.values().collect::<Vec<_>>();
    //     players.sort_by(|a, b| a.position.y.partial_cmp(&b.position.y).unwrap());

    //     for player in players {
    //         player.draw(&mut draw, time, &mut self.assets);
    //     }

    //     self.map.draw_layers(&mut draw, &[MapLayer::Fringe, MapLayer::Fringe2], &mut self.assets, time);

    //     if self.ui.map_editor_shown {
    //         self.map.draw_zones(&mut draw, &mut self.assets);
    //         if let Some(drag_start) = self.ui.drag_start {
    //             let mouse = self.camera.screen_to_world(mouse_position.into());
    //             let start = drag_start.min(mouse);
    //             let end = drag_start.max(mouse);
    //             let size = end - start;

    //             let drag_rect = rect(start.x, start.y, size.x, size.y);
    //             draw_zone(&mut draw, drag_rect, self.ui.map_editor.zone_data(), &mut self.assets);
    //         }
    //     }

    //     draw.transform().pop();

    //     draw_text_shadow(
    //         &mut draw,
    //         &self.assets.font.lock().unwrap(),
    //         &self.map.settings.name,
    //         vec2(screen_width as f32 / 2.0, 2.0),
    //         |text| {
    //             text.color(Color::WHITE)
    //                 .h_align_center();
    //         }
    //     );

    //     draw.text(&self.assets.debug_font, &format!("{:.02}", ctx.app.timer.fps()))
    //         .position(0., 0.)
    //         .size(16.)
    //         .color(Color::WHITE);

    //     ctx.gfx.render(&draw);

    //     let screen_size = ivec2(screen_width, screen_height);
    //     let mouse_world_position = self.camera.screen_to_world(mouse_position.into());

    //     let output = ctx.plugins.egui(|ui_ctx| {
    //         self.draw_ui(ui_ctx, screen_size, mouse_world_position);
    //         self.ui.block_keyboard = ui_ctx.wants_keyboard_input();
    //         self.ui.block_pointer = ui_ctx.wants_pointer_input();
    //     });

    //     ctx.gfx.render(&output);
    // }

    // fn update(&mut self, ctx: &mut UpdateContext) {
    //     self.update_network(ctx);

    //     self.update_players(ctx);

    //     self.update_input(ctx);
    //     self.update_camera(ctx);

    //     let size = ctx.app.window().size();
    //     self.camera.update_matrix(vec2(size.0 as f32, size.1 as f32));
    // }

    // fn event(&mut self, ctx: &mut EventContext) {
    //     if ctx.event == Event::Exit {
    //         log::info!("Goodbye!");
    //         self.network.stop();
    //     }
    // }
}

struct UiState {
    block_keyboard: bool,
    block_pointer: bool,
    chat_window: ChatWindow,
    drag_zone: Option<(Point2D, Size2D)>,
    last_tile: Option<(MouseButton, map::Point2D)>,
    map_editor_shown: bool,
    map_editor: MapEditor,
}

impl UiState {
    pub fn drag_box2d(&self) -> Option<Box2D> {
        self.drag_zone.map(|(drag_start, drag_size)| {
            let min = Point2D::min(drag_start, drag_start + drag_size);
            let max = Point2D::max(drag_start, drag_start + drag_size);

            Box2D::new(min, max)
        })
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            block_keyboard: false,
            block_pointer: false,
            chat_window: ChatWindow::new(),
            drag_zone: None,
            last_tile: None,
            map_editor_shown: false,
            map_editor: MapEditor::new(),
        }
    }
}

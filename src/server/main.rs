mod data;
mod player;

use std::{
    collections::{hash_map::Entry, HashMap}, path::PathBuf, time::{Duration, Instant}
};

use anyhow::{bail, Context, Result};
use base64ct::{Base64, Encoding};
use euclid::size2;
use onyx::{
    network::{
        client::Packet as ClientPacket, server::{FailJoinReason, Packet}, ChatChannel, Direction, Entity, MapId, Zone, ZoneData
    }, SPRITE_SIZE
};
use onyx::math::units::world::*;
use env_logger::WriteStyle;
use log::LevelFilter;
use network::Network;
use rand::prelude::*;
use renet::{ClientId, DefaultChannel, ServerEvent};
use sha2::{Digest, Sha256};

use crate::data::{Config, Map, NameCache, Player};

mod network;

fn main() -> Result<()> {
    if let Ok(runtime) = std::env::var("RUNTIME_PATH") {
        let runtime = PathBuf::from(runtime).join("server");
        log::warn!("Setting runtime to {}", runtime.display());
        std::env::set_current_dir(runtime).unwrap();
    }

    #[cfg(debug_assertions)]
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .write_style(WriteStyle::Always)
        .init();

    let game_server = GameServer::new()?;
    game_server.run();
    Ok(())
}

#[derive(Copy, Clone, Default, Debug)]
struct WarpParams {
    initial: bool,
    position: Option<Point2D>,
    direction: Option<Direction>,
}

struct GameServer {
    network: Network,
    config: Config,
    players: HashMap<Entity, Player>,
    maps: HashMap<MapId, Map>,
    start: Instant,
    time_since_start: Duration,
    next_entity: u64,
    /// Time since last update
    dt: Duration,
    _rng: ThreadRng,
}

impl GameServer {
    pub fn new() -> Result<Self> {
        let config = Config::load().context("load config")?;
        let mut maps = Map::load_all().context("load maps")?;

        if let Entry::Vacant(e) = maps.entry(MapId::default()) {
            e.insert(create_map(MapId::default()));
        }

        let network = Network::listen(&config);

        Ok(Self {
            network,
            config,
            players: HashMap::new(),
            start: Instant::now(),
            time_since_start: Duration::ZERO,
            dt: Duration::ZERO,
            next_entity: 0,
            maps,
            _rng: rand::thread_rng(),
        })
    }

    fn run(mut self) {
        loop {
            let since_start = self.start.elapsed();
            let dt = since_start - self.time_since_start;

            self.time_since_start = since_start;
            self.dt = dt;

            self.update_network();
            self.update_tick();

            // finalizing
            self.maintain();

            std::thread::sleep(Duration::from_secs_f64(1.0 / 30.0));
        }
    }

    fn update_network(&mut self) {
        self.network.server.update(self.dt);
        self.network.transport.update(self.dt, &mut self.network.server).unwrap();

        while let Some(event) = self.network.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    self.network.client_ids.insert(client_id);
                    log::info!(
                        "Client ({}) connected (total clients: {})",
                        client_id,
                        self.network.client_ids.len()
                    );
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    self.network.client_ids.remove(&client_id);
                    self.handle_disconnect(client_id, reason.to_string());
                }
            }
        }

        for client_id in self.network.server.clients_id() {
            while let Some(bytes) = self.network.server.receive_message(client_id, DefaultChannel::ReliableUnordered) {
                if let Ok(message) = rmp_serde::from_slice(&bytes) {
                    if let Err(e) = self.handle_message(client_id, message) {
                        self.network.server.disconnect(client_id);
                        log::warn!(
                            "Disconnecting client ({}), message handler returned an error: {e}",
                            client_id,
                        );
                    }
                }
            }
        }

        self.network.transport.send_packets(&mut self.network.server);

        // while let Some(event) = receive.try_receive() {
        //     match event.network() {
        //         StoredNetEvent::Connected(_, _) => unreachable!(),
        //         StoredNetEvent::Accepted(client_id, _listener) => {
        //             self.client_ids.insert(client_id);

        //             log::info!(
        //                 "Client ({}) connected (total clients: {})",
        //                 client_id.addr(),
        //                 self.client_ids.len()
        //             );
        //         }
        //         StoredNetEvent::Message(client_id, bytes) => {
        //             let message = rmp_serde::from_slice(&bytes).unwrap();

        //             if let Err(e) = self.handle_message(client_id, message) {
        //                 self.client_ids.remove(&client_id);
        //                 self.handle_disconnect(client_id);
        //                 log::warn!(
        //                     "Disconnecting client ({}), message handler returned an error: {e}",
        //                     client_id.addr(),
        //                 );
        //             }
        //         }
        //         StoredNetEvent::Disconnected(client_id) => {
        //             self.client_ids.remove(&client_id);
        //             self.handle_disconnect(client_id);
        //         }
        //     }
        // }
    }

    fn handle_disconnect(&mut self, client_id: ClientId, reason: String) {
        log::info!(
            "Client ({}) disconnected (total clients: {}): {}",
            client_id,
            self.network.client_ids.len(),
            reason
        );

        let Some(&entity) = self.network.peer_map.get_by_right(&client_id) else {
            return;
        };

        if let Some(player) = self.players.remove(&entity) {
            let players: Vec<_> = self.players
                .iter()
                .filter_map(|(&id, data)| (data.map == player.map).then_some(id))
                .collect();

            self.network.send_list(&players, &Packet::RemoveData(entity));

            let goodbye = Packet::ChatLog(ChatChannel::Server, format!("{} has left the game.", &player.name));
            self.network.broadcast_except(entity, &goodbye);

            player.save().unwrap();
        }

        self.network.peer_map.remove_by_right(&client_id);
    }

    fn handle_message(&mut self, client_id: ClientId, message: ClientPacket) -> Result<()> {
        use onyx::network::client::Packet::*;

        match &message {
            SaveMap(_) => log::debug!("{}: SaveMap(..)", client_id),
            message => {
                log::debug!("{}: {:?}", client_id, message);
            }
        }

        if let Some(&entity) = self.network.peer_map.get_by_right(&client_id) {
            self.handle_game_message(entity, message)
        } else {
            self.handle_login_message(client_id, message)
        }
    }

    fn handle_login_message(&mut self, client_id: ClientId, message: ClientPacket) -> Result<()> {
        match message {
            ClientPacket::CreateAccount {
                username,
                password,
                character_name: name,
            } => {
                let mut name_cache = NameCache::load().unwrap();

                if Player::path(&username).exists() {
                    self.network.send_to(client_id, &Packet::FailedJoin(FailJoinReason::UsernameTaken));
                }

                if name_cache.contains(&name) {
                    self.network.send_to(client_id, &Packet::FailedJoin(FailJoinReason::CharacterNameTaken));
                }

                name_cache.insert(name.clone());
                name_cache.save().unwrap();

                let hash = Sha256::digest(password);
                let password = Base64::encode_string(&hash);

                let player = Player {
                    username,
                    password,
                    name,
                    position: Point2D::new(self.config.start.x, self.config.start.y),
                    ..Default::default()
                };

                player.save().unwrap();

                self.join_game(client_id, player);
            }
            ClientPacket::Login { username, password } => {
                if let Some(player) = Self::check_password(&username, &password) {
                    self.join_game(client_id, player);
                } else {
                    let incorrect = Packet::FailedJoin(FailJoinReason::LoginIncorrect);
                    self.network.send_to(client_id, &incorrect);
                }
            }
            _ => {
                bail!("Client attempted to send a packet that is invalid while logged in");
            }
        }

        Ok(())
    }

    fn check_password(username: &str, password: &str) -> Option<Player> {
        match Player::load(username) {
            Ok(player) => {
                let hash = Sha256::digest(password);
                let password = Base64::encode_string(&hash);
                if player.password == password {
                    return Some(player);
                } else {
                    log::warn!("Failed to log in, passwords do not match");
                }
            }
            Err(e) => {
                log::warn!("Failed to log in, loading player errored: {e}");
            }
        }
        None
    }

    fn handle_game_message(&mut self, entity: Entity, message: ClientPacket) -> Result<()> {
        match message {
            ClientPacket::CreateAccount { .. } | ClientPacket::Login { .. } => unreachable!(),

            ClientPacket::ChatMessage(channel, text) => {
                self.process_chat_message(entity, channel, &text);
            }
            ClientPacket::RequestMap => {
                let map = &self.maps[&self.players[&entity].map];

                let packet = Packet::MapData(Box::new(map.clone().into()));
                self.network.send(entity, &packet);
            }
            ClientPacket::SaveMap(map) => {
                use std::time::SystemTime;

                let map_id = self.players[&entity].map;
                let mut map = Map::from(*map);

                map.settings.cache_key = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                    Ok(n) => n.as_secs(),
                    Err(_) => unreachable!()
                };

                if let Err(e) = map.save() {
                    log::error!("Couldn't save map {e}");
                }

                self.maps.insert(map_id, map.clone());
                let map_list = self.entities_on_map(map_id);
                self.network.send_list(&map_list, &Packet::MapData(Box::new(map.into())));
            }
            ClientPacket::Move {
                position,
                velocity,
            } => {
                let map_id = self.players[&entity].map;
                let map = &self.maps[&map_id];

                let valid = check_collision_with(
                    position,
                    map.zones.iter().filter(|zone| zone.data == ZoneData::Blocked),
                    |zone| Box2D::from_origin_and_size(zone.position, zone.size),
                )
                .is_none();

                if valid {
                    let player = self.players.get_mut(&entity).unwrap();

                    player.position = position;
                    player.velocity = velocity;

                    // let packet = Packet::PlayerMove {
                    //     entity,
                    //     position,
                    //     direction,
                    //     velocity,
                    // };

                    // self.send_map_except(map_id, entity, &packet);
                } else {
                    // warping them to the default will just update them with the server truth
                    self.warp_player(entity, map_id, WarpParams::default());
                }
            }
            ClientPacket::Warp(map_id, position) => {
                // note: the requested map possibly doesn't exist
                self.validate_map(map_id);

                self.warp_player(
                    entity,
                    map_id,
                    WarpParams {
                        position,
                        direction: Some(Direction::South),
                        ..Default::default()
                    },
                );

                // above warp *needs* to create the map or this will fail
                self.send_map_editor(entity, map_id)?;
            }
            ClientPacket::MapEditor(open) => {
                let player = self.players.get_mut(&entity).unwrap();
                player.flags.in_map_editor = open;

                // borrow checker go to hell
                let player = self.players.get(&entity).unwrap();
                let map_id = player.map;
                let flags = player.flags;

                let entities = self.entities_on_map(map_id);
                self.network.send_list(&entities, &Packet::Flags(entity, flags));

                if open {
                    self.send_map_editor(entity, map_id)?;
                }
            }
        }

        Ok(())
    }

    fn process_chat_message(&mut self, entity: Entity, channel: ChatChannel, message: &str) {
        if let Some(command) = self.process_chat_command(entity, message) {
            log::info!("{entity:?}: used the {command} command");
            return;
        }

        let player = &self.players[&entity];
        match channel {
            ChatChannel::Echo | ChatChannel::Error => {
                log::warn!("Client tried to talk in an invalid channel");
            }
            ChatChannel::Server => {
                let packet = Packet::ChatLog(ChatChannel::Server, message.to_string());
                self.network.broadcast(&packet);
            }
            ChatChannel::Say => {
                let full_text = format!("{}: {}", player.name, message);

                let packet = Packet::ChatLog(ChatChannel::Say, full_text);
                let entities = self.entities_on_map(player.map);
                self.network.send_list(&entities, &packet);
            }
            ChatChannel::Global => {
                let full_text = format!("{}: {}", player.name, message);
                let packet = Packet::ChatLog(ChatChannel::Global, full_text);
                self.network.broadcast(&packet);
            }
        }
    }

    fn process_chat_command(&mut self, entity: Entity, message: &str) -> Option<&str> {
        if let Some(args) = message.strip_prefix("/warp") {
            let parsed = args.trim().parse::<u64>().map(MapId::from);
            if let Ok(map_id) = parsed {
                self.validate_map(map_id);
                self.warp_player(
                    entity,
                    map_id,
                    WarpParams {
                        direction: Some(Direction::South),
                        ..Default::default()
                    },
                );
            } else {
                self.network.send(
                    entity,
                    &Packet::ChatLog(ChatChannel::Error, String::from("Usage: /warp <map id>")),
                );
            }

            return Some("/warp");
        }

        if let Some(args) = message.strip_prefix("/sprite") {
            // wtb try blocks
            let result: Result<(), &str> = (|| {
                let (who, sprite) = args.trim().rsplit_once(' ').ok_or("Wrong number of arguments")?;
                let sprite = sprite.parse().map_err(|_| "Invalid sprite, it must be a number")?;

                let other_entity = *self.players.iter()
                    .find_map(|(id, player)| (player.name == who).then_some(id))
                    .ok_or("Could not find the player, are they online?")?;

                let player = self.players.get_mut(&other_entity).unwrap();
                player.sprite = sprite;
                if let Err(e) = player.save() {
                    log::error!("Couldn't save player: {e}");
                };

                let player = self.players.get(&other_entity).unwrap();
                let packet = Packet::PlayerData(other_entity, player.clone().into());
                let entities = self.entities_on_map(player.map);
                self.network.send_list(&entities, &packet);

                Ok(())
            })();

            if let Err(e) = result {
                let error = format!("Error: {}\nUsage: /sprite <player name> <sprite #>", e);
                self.network.send(entity, &Packet::ChatLog(ChatChannel::Error, error));
            }

            return Some("/sprite");
        }

        None
    }

    fn send_map_editor(&mut self, entity: Entity, map_id: MapId) -> Result<()> {
        let maps = self
            .maps
            .values()
            .map(|map| (map.id, map.settings.name.clone()))
            .collect::<HashMap<_, _>>();

        let map = self.maps.get(&map_id)
            .expect("attempting to send map editor for a map that doesnt exist");

        let id = map.id;
        let width = map.size.width;
        let height = map.size.height;
        let settings = map.settings.clone();

        self.network.send(
            entity,
            &Packet::MapEditor {
                maps,
                id,
                width,
                height,
                settings,
            },
        );

        Ok(())
    }

    fn join_game(&mut self, client_id: ClientId, mut player: Player) {
        // Allocate an entity with an associated client_id
        let entity = self.next_entity();
        self.network.peer_map.insert(entity, client_id);

        // Make sure they're on a valid map, and if they're not move them.
        if !self.maps.contains_key(&player.map) {
            player.map = MapId::default();
            player.position = self.config.start.position();
        }

        // Save their data
        self.players.insert(entity, player.clone());

        // Send them their ID
        self.network.send(entity, &Packet::JoinGame(entity));

        self.warp_player(
            entity,
            player.map,
            WarpParams {
                initial: true,
                ..Default::default()
            },
        );

        // Send welcome message
        self.network.send(
            entity,
            &Packet::ChatLog(ChatChannel::Server, "Welcome to Game™!".to_owned()),
        );

        // Send join message
        self.network.broadcast_except(
            entity,
            &Packet::ChatLog(ChatChannel::Server, format!("{} has joined the game.", &player.name)),
        );
    }

    fn update_tick(&mut self) {
        self.update_players();
    }

    fn update_players(&mut self) {
        // let dt = self.dt;

        let mut to_warp = Vec::new();
        let mut position_updates = Vec::new();

        // let sprite_size = Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0);
        // let sprite_offset = Vector2D::new(0.0, SPRITE_SIZE as f32 / 2.0);

        // let player_boxes = self
        //     .players
        //     .iter()
        //     .map(|(&entity, player)| {
        //         (
        //             entity,
        //             Box2D::from_origin_and_size(player.position + sprite_offset, sprite_size),
        //         )
        //     })
        //     .collect::<Vec<_>>();

        for (&entity, player) in &mut self.players {
            let map = &self.maps[&player.map];
        //     if let Some(velocity) = player.velocity {
        //         let offset = velocity * dt.as_secs_f32();
        //         let new_position = player.position + offset;
        //         let mut valid = true;

        //         // map bounds
        //         if !player.flags.in_map_editor {
        //             // map warps, lots of copy paste code lol
        //             if let Some((direction, new_position)) = check_edge_warp(map, new_position) {
        //                 let map_id = match direction {
        //                     Direction::North => map.settings.warps.north,
        //                     Direction::South => map.settings.warps.south,
        //                     Direction::East => map.settings.warps.east,
        //                     Direction::West => map.settings.warps.west,
        //                 };

        //                 if let Some(map_id) = map_id {
        //                     to_warp.push((
        //                         entity,
        //                         map_id,
        //                         WarpParams {
        //                             position: Some(new_position),
        //                             ..Default::default()
        //                         },
        //                     ));
        //                 }

        //                 valid = false;
        //             }
        //         }

        //         if !player.flags.in_map_editor {
        //             // block zones
        //             valid &= check_collision_with(
        //                 player.position,
        //                 map.zones.iter().filter(|zone| zone.data == ZoneData::Blocked),
        //                 |zone| Box2D::from_origin_and_size(zone.position.into(), zone.size.into()),
        //             )
        //             .is_none();

        //             valid &= check_collision_with(
        //                 player.position,
        //                 player_boxes.iter().filter(|(id, _box2d)| *id != entity),
        //                 |(_id, box2d)| *box2d,
        //             )
        //             .is_none();
        //         }

        //         if valid {
        //             player.position = new_position;
        //         }
        //     }

            if !player.flags.in_map_editor {
                let warp = check_collision_with(
                    player.position,
                    map.zones
                        .iter()
                        .filter(|zone| matches!(zone.data, ZoneData::Warp(_, _, _))),
                    |zone| Box2D::from_origin_and_size(zone.position, zone.size),
                );

                if let Some(Zone {
                    data: ZoneData::Warp(map_id, position, direction),
                    ..
                }) = warp {
                    to_warp.push((
                        entity,
                        *map_id,
                        WarpParams {
                            position: Some(*position),
                            direction: *direction,
                            ..Default::default()
                        },
                    ));
                }
            }

             // TODO: Check for dirty
             const UPDATE_RATE: f32 = 1.0 / 20.0;
             if self.time_since_start.as_secs_f32() >= player.last_movement_update + UPDATE_RATE {
                 let packet = Packet::PlayerMove {
                     entity,
                     position: player.position,
                     velocity: player.velocity,
                 };
     
                 position_updates.push((player.map, entity, packet));
             }
        }

        // TODO Map-warp ordering?
        for (map, entity, packet) in position_updates {
            let entities = self.entities_on_map_except(map, entity);
            self.network.send_list(&entities, &packet);
        }

        for (entity, map_id, params) in to_warp {
            self.validate_map(map_id);
            self.warp_player(entity, map_id, params);
        }
    }

    // Convenience function to validate that a map exists by it's name, and then return it's hash
    fn validate_map(&mut self, map_id: MapId) {
        if let Entry::Vacant(e) = self.maps.entry(map_id) {
            e.insert(create_map(map_id));
        }
    }

    /// Warps the player to a specific map, sending all the correct packets
    fn warp_player(&mut self, entity: Entity, map_id: MapId, params: WarpParams) {
        if !self.players.contains_key(&entity) {
            return;
        }

        let old_map = self.players[&entity].map;

        // check if we're actually changing maps, or if we're just moving to a new position.
        if params.initial || self.players[&entity].map != map_id {
            if !params.initial {
                let entities = self.entities_on_map_except(old_map, entity);
                self.network.send_list(&entities, &Packet::RemoveData(entity));
            }

            self.players.get_mut(&entity).unwrap().map = map_id;
            let cache_key = self.maps[&map_id].settings.cache_key;

            self.network.send(entity, &Packet::ChangeMap(map_id, cache_key));

            let packets = self
                .players
                .iter()
                .filter(|(_, player_data)| player_data.map == map_id)
                .map(|(&id, data)| Packet::PlayerData(id, data.clone().into()))
                .collect::<Vec<_>>();

            for packet in packets {
                self.network.send(entity, &packet);
            }

            let player_data = self.players[&entity].clone();
            let packet = Packet::PlayerData(entity, player_data.into());
            let entities = self.entities_on_map(map_id);
            self.network.send_list(&entities, &packet);
        }

        if let Some(player) = self.players.get_mut(&entity) {
            if let Some(position) = params.position {
                player.position = position;
            }
            if let Some(direction) = params.direction {
                player.direction = direction;
                player.velocity = Vector2D::zero();
            }

            let packet = Packet::PlayerMove {
                entity,
                position: player.position,
                velocity: player.velocity,
            };

            player.save().unwrap();
            let entities = self.entities_on_map(map_id);
            self.network.send_list(&entities, &packet);
        }
    }

    pub fn maintain(&mut self) {
        // lol
    }

    fn next_entity(&mut self) -> Entity {
        let idx = self.next_entity;
        self.next_entity += 1;

        Entity(idx)
    }

    fn entities_on_map(&self, map_id: MapId) -> Vec<Entity> {
        self.players.iter()
            .filter(move |(_entity, player)| player.map == map_id)
            .map(|(entity, _player)| *entity)
            .collect()
    }

    fn entities_on_map_except(&self, map_id: MapId, exclude: Entity) -> Vec<Entity>  {
        self.players.iter()
            .filter(move |(entity, player)| player.map == map_id && **entity != exclude)
            .map(|(entity, _player)| *entity)
            .collect()
    }
}

/// Network convenience


fn check_bounds(position: Point2D, bounds: Box2D) -> Option<Direction> {
    let bounds = bounds.to_rect();
    let sprite = sprite_box(position);

    if sprite.min.y <= bounds.min_y() {
        Some(Direction::North)
    } else if sprite.max.y >= bounds.max_y() {
        Some(Direction::South)
    } else if sprite.min.x <= bounds.min_x() {
        Some(Direction::West)
    } else if sprite.max.x >= bounds.max_x() {
        Some(Direction::East)
    } else {
        None
    }
}

fn check_collision_with<'a, T>(
    position: Point2D,
    mut blockers: impl Iterator<Item = &'a T>,
    map_with: impl Fn(&'a T) -> Box2D,
) -> Option<&'a T> {
    let sprite = sprite_box(position);

    blockers.find(|item| sprite.intersects(&map_with(item)))
}

fn check_edge_warp(map: &Map, position: Point2D) -> Option<(Direction, Point2D)> {
    let map_box = map.to_box2d();

    if let Some(direction) = check_bounds(position, map_box) {
        let map_rect = map_box.to_rect();
        let new_position = match direction {
            Direction::North => Point2D::new(position.x, map_rect.max_y() - SPRITE_SIZE),
            Direction::South => Point2D::new(position.x, -SPRITE_SIZE / 2.0),
            Direction::West => Point2D::new(map_rect.max_x() - SPRITE_SIZE, position.y),
            Direction::East => Point2D::new(0.0, position.y),
        };

        Some((direction, new_position))
    } else {
        None
    }
}

fn create_map(id: MapId) -> Map {
    let map = Map::new(id, size2(20, 15));
    map.save().unwrap();
    map
}

fn sprite_box(position: Point2D) -> Box2D {
    const SPRITE_SIZE2D: Size2D = Size2D::new(SPRITE_SIZE, SPRITE_SIZE / 2.0);
    const SPRITE_OFFSET: Vector2D = Vector2D::new(0.0, SPRITE_SIZE / 2.0);

    Box2D::from_origin_and_size(position + SPRITE_OFFSET, SPRITE_SIZE2D)
}

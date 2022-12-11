mod data;
mod player;

use std::{
    collections::{hash_map::Entry, HashMap},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::Utc;
use common::{
    network::{
        client::Packet as ClientPacket,
        server::{FailJoinReason, Packet},
        ChatChannel, ClientId, Direction, Zone, ZoneData,
    },
    SPRITE_SIZE, START_MAP,
};
use env_logger::WriteStyle;
use euclid::default::{Box2D, Point2D, Size2D, Vector2D};
use log::LevelFilter;
use message_io::{
    network::{Endpoint, NetworkController, Transport},
    node::{self, NodeHandler, StoredNetEvent},
};
use rand::prelude::*;
use sha2::{Digest, Sha256};

use crate::data::{Config, Map, NameCache, Player};

fn main() -> Result<()> {
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
    position: Option<Point2D<f32>>,
    direction: Option<Direction>,
}

struct GameServer {
    config: Config,
    players: HashMap<ClientId, Player>,
    peer_map: HashMap<ClientId, Endpoint>,
    maps: HashMap<String, Map>,
    time: Instant,
    /// Time since last update
    dt: Duration,
    handler: Option<NodeHandler<()>>,
    rng: ThreadRng,
}

impl GameServer {
    pub fn new() -> Result<Self> {
        let config = Config::load().context("load config")?;
        let mut maps = Map::load_all().context("load maps")?;

        if let Entry::Vacant(e) = maps.entry(START_MAP.to_string()) {
            e.insert(create_map(START_MAP));
        }

        Ok(Self {
            config,
            players: HashMap::new(),
            peer_map: HashMap::new(),
            time: Instant::now(),
            dt: Duration::ZERO,
            handler: None,
            maps,
            rng: rand::thread_rng(),
        })
    }

    fn run(mut self) {
        let (handler, listener) = node::split::<()>();
        handler
            .network()
            .listen(Transport::FramedTcp, self.config.listen.clone())
            .unwrap();

        self.handler = Some(handler);

        log::info!("Listening on {}", self.config.listen);

        let (_task, mut receive) = listener.enqueue();

        let mut peer_map: HashMap<Endpoint, ClientId> = HashMap::new();
        let mut idx = 0u64;

        loop {
            let now = Instant::now();
            let dt = now - self.time;
            self.time = now;
            self.dt = dt;

            if let Some(event) = receive.try_receive() {
                match event.network() {
                    StoredNetEvent::Connected(_, _) => unreachable!(),
                    StoredNetEvent::Accepted(endpoint, _listener) => {
                        let client_id = ClientId(idx);
                        peer_map.insert(endpoint, client_id);
                        self.peer_map.insert(client_id, endpoint);
                        idx += 1;

                        log::info!(
                            "Client ({}) connected (total clients: {})",
                            endpoint.addr(),
                            peer_map.len()
                        );
                    }
                    StoredNetEvent::Message(endpoint, bytes) => {
                        let message = rmp_serde::from_slice(&bytes).unwrap();
                        let client_id = peer_map[&endpoint];

                        if let Err(e) = self.handle_message(client_id, message) {
                            if let Some(client_id) = peer_map.remove(&endpoint) {
                                self.peer_map.remove(&client_id);
                                self.handle_disconnect(client_id);
                            }

                            log::warn!(
                                "Disconnecting client ({}), message handler returned an error: {e}",
                                endpoint.addr(),
                            );

                            log::info!(
                                "Client ({}) disconnected (total clients: {})",
                                endpoint.addr(),
                                peer_map.len()
                            );
                        }
                    }
                    StoredNetEvent::Disconnected(endpoint) => {
                        if let Some(client_id) = peer_map.remove(&endpoint) {
                            self.peer_map.remove(&client_id);
                            self.handle_disconnect(client_id);
                        }

                        log::info!(
                            "Client ({}) disconnected (total clients: {})",
                            endpoint.addr(),
                            peer_map.len()
                        );
                    }
                }
            }

            // game loop
            self.tick();

            // finalizing
            self.maintain();
            std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0));
        }
    }

    fn handle_disconnect(&mut self, client_id: ClientId) {
        if let Some(player) = self.players.remove(&client_id) {
            self.send_list(
                &self
                    .players
                    .iter()
                    .filter(|(_, data)| data.map == player.map)
                    .map(|(&cid, _)| cid)
                    .collect::<Vec<_>>(),
                &Packet::RemoveData(client_id),
            );

            let goodbye = Packet::ChatLog(ChatChannel::Server, format!("{} has left the game.", &player.name));
            self.send_exclude(client_id, &goodbye);

            player.save().unwrap();
        }
    }

    fn handle_message(&mut self, client_id: ClientId, message: ClientPacket) -> Result<()> {
        use common::network::client::Packet::*;

        match &message {
            SaveMap(_) => log::debug!("SaveMap(..)"),
            message => {
                log::debug!("{client_id:?}: {message:?}");
            }
        }

        if self.players.contains_key(&client_id) {
            self.handle_game_message(client_id, message)
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
                    self.send(client_id, &Packet::FailedJoin(FailJoinReason::UsernameTaken));
                }

                if name_cache.contains(&name) {
                    self.send(client_id, &Packet::FailedJoin(FailJoinReason::CharacterNameTaken));
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
                    map: START_MAP.to_string(),
                    ..Default::default()
                };

                player.save().unwrap();

                self.join_game(client_id, player);
            }
            ClientPacket::Login { username, password } => {
                let incorrect = Packet::FailedJoin(FailJoinReason::LoginIncorrect);

                if let Some(player) = Self::check_password(&username, &password) {
                    self.join_game(client_id, player);
                } else {
                    self.send(client_id, &incorrect);
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

    fn handle_game_message(&mut self, client_id: ClientId, message: ClientPacket) -> Result<()> {
        match message {
            ClientPacket::CreateAccount { .. } | ClientPacket::Login { .. } => unreachable!(),

            ClientPacket::ChatMessage(channel, text) => {
                self.process_chat_message(client_id, channel, &text);
            }
            ClientPacket::RequestMap => {
                let map = &self.maps[&self.players[&client_id].map];

                let packet = Packet::MapData(Box::new(map.clone().into()));
                self.send(client_id, &packet);
            }
            ClientPacket::SaveMap(map) => {
                let map_id: &str = self.players[&client_id].map.as_ref();
                let mut map = Map::from(*map);
                
                map.settings.cache_key = Utc::now().timestamp_millis();

                if let Err(e) = map.save() {
                    log::error!("Couldn't save map {e}");
                }

                self.maps.insert(map_id.to_string(), map.clone());
                self.send_to_map(map_id, &Packet::MapData(Box::new(map.into())));
            }
            ClientPacket::Move {
                position,
                direction,
                velocity,
            } => {
                let map_id = self.players[&client_id].map.clone();
                let map = &self.maps[&map_id];

                let valid = check_collision_with(
                    position.into(),
                    map.zones.iter().filter(|zone| zone.data == ZoneData::Blocked),
                    |zone| Box2D::from_origin_and_size(zone.position.into(), zone.size.into()),
                )
                .is_none();

                if valid {
                    let player = self.players.get_mut(&client_id).unwrap();

                    player.position = position.into();
                    player.velocity = velocity.map(Into::into);
                    player.direction = direction;

                    let packet = Packet::PlayerMove {
                        client_id,
                        position,
                        direction,
                        velocity,
                    };

                    self.send_map_except(&map_id, client_id, &packet);
                } else {
                    // warping them to the default will just update them with the server truth
                    self.warp_player(client_id, &map_id, WarpParams::default());
                }
            }
            ClientPacket::Warp(map_id, position) => {
                // note: the requested map possibly doesn't exist
                self.validate_map(&map_id);

                self.warp_player(
                    client_id,
                    &map_id,
                    WarpParams {
                        position: position.map(Into::into),
                        direction: Some(Direction::South),
                        ..Default::default()
                    },
                );

                // above warp *needs* to create the map or this will fail
                self.send_map_editor(client_id, &map_id)?;
            }
            ClientPacket::MapEditor(open) => {
                let player = self.players.get_mut(&client_id).unwrap();
                player.flags.in_map_editor = open;

                // borrow checker go to hell
                let player = self.players.get(&client_id).unwrap();
                let map_id = player.map.as_ref();
                let flags = player.flags;

                self.send_to_map(map_id, &Packet::Flags(client_id, flags));

                if open {
                    self.send_map_editor(client_id, map_id)?;
                }
            }
        }

        Ok(())
    }

    fn process_chat_message(&mut self, client_id: ClientId, channel: ChatChannel, message: &str) {
        if let Some(command) = self.process_chat_command(client_id, message) {
            log::info!("{client_id:?}: used the {command} command");
            return;
        }

        let player = &self.players[&client_id];
        match channel {
            ChatChannel::Echo | ChatChannel::Error => {
                log::warn!("Client tried to talk in an invalid channel");
            }
            ChatChannel::Server => {
                let packet = Packet::ChatLog(ChatChannel::Server, message.to_string());
                self.send_all(&packet);
            }
            ChatChannel::Say => {
                let full_text = format!("{}: {}", player.name, message);

                let packet = Packet::ChatLog(ChatChannel::Say, full_text);
                self.send_to_map(&player.map, &packet);
            }
            ChatChannel::Global => {
                let full_text = format!("{}: {}", player.name, message);
                let packet = Packet::ChatLog(ChatChannel::Global, full_text);
                self.send_all(&packet);
            }
        }
    }

    fn process_chat_command(&mut self, client_id: ClientId, message: &str) -> Option<&str> {
        if let Some(args) = message.strip_prefix("/warp") {
            let map_id = args.trim();
            if !map_id.is_empty() {
                self.validate_map(map_id);
                self.warp_player(
                    client_id,
                    map_id,
                    WarpParams {
                        direction: Some(Direction::South),
                        ..Default::default()
                    },
                );
            } else {
                self.send(
                    client_id,
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

                let other_id = *self
                    .players
                    .iter()
                    .find_map(|(cid, player)| (player.name == who).then_some(cid))
                    .ok_or("Could not find the player, are they online?")?;

                let mut player = self.players.get_mut(&other_id).unwrap();
                player.sprite = sprite;
                if let Err(e) = player.save() {
                    log::error!("Couldn't save player: {e}");
                };

                let player = self.players.get(&other_id).unwrap();
                self.send_to_map(&player.map, &Packet::PlayerData(other_id, player.clone().into()));

                Ok(())
            })();

            if let Err(e) = result {
                let error = format!("Error: {}\nUsage: /sprite <player name> <sprite #>", e);
                self.send(client_id, &Packet::ChatLog(ChatChannel::Error, error));
            }

            return Some("/sprite");
        }

        None
    }

    fn send_map_editor(&self, client_id: ClientId, map_id: &str) -> Result<()> {
        let maps = self
            .maps
            .values()
            .map(|map| (map.id.clone(), map.settings.name.clone()))
            .collect::<HashMap<_, _>>();

        let map = &self.maps[map_id];

        let id = map.id.to_string();
        let width = map.width;
        let height = map.height;
        let settings = map.settings.clone();

        self.send(
            client_id,
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
        // Make sure they're on a valid map, and if they're not move them.
        if !self.maps.contains_key(&player.map) {
            player.map = START_MAP.to_string();
            player.position = self.config.start.position();
        }

        // Save their data
        self.players.insert(client_id, player.clone());

        // Send them their ID
        self.send(client_id, &Packet::JoinGame(client_id));

        self.warp_player(
            client_id,
            &player.map,
            WarpParams {
                initial: true,
                ..Default::default()
            },
        );

        // Send welcome message
        self.send(
            client_id,
            &Packet::ChatLog(ChatChannel::Server, "Welcome to Game™!".to_owned()),
        );

        // Send join message
        self.send_exclude(
            client_id,
            &Packet::ChatLog(ChatChannel::Server, format!("{} has joined the game.", &player.name)),
        );
    }

    fn tick(&mut self) {
        self.update_players();
    }

    fn update_players(&mut self) {
        let dt = self.dt;

        let mut to_warp = Vec::new();

        let sprite_size = Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0);
        let sprite_offset = Vector2D::new(0.0, SPRITE_SIZE as f32 / 2.0);

        let player_boxes = self
            .players
            .iter()
            .map(|(client_id, player)| {
                (
                    *client_id,
                    Box2D::from_origin_and_size(player.position + sprite_offset, sprite_size),
                )
            })
            .collect::<Vec<_>>();

        for (client_id, player) in &mut self.players {
            let map = &self.maps[&player.map];
            if let Some(velocity) = player.velocity {
                let offset = velocity * dt.as_secs_f32();
                let new_position = player.position + offset;
                let mut valid = true;

                // map bounds
                if !player.flags.in_map_editor {
                    // map warps, lots of copy paste code lol
                    if let Some((direction, new_position)) = check_edge_warp(map, new_position) {
                        let map_id = match direction {
                            Direction::North => map.settings.warps.north.clone(),
                            Direction::South => map.settings.warps.south.clone(),
                            Direction::East => map.settings.warps.east.clone(),
                            Direction::West => map.settings.warps.west.clone(),
                        };

                        if let Some(map_id) = map_id {
                            to_warp.push((
                                *client_id,
                                map_id,
                                WarpParams {
                                    position: Some(new_position),
                                    ..Default::default()
                                },
                            ));
                        }

                        valid = false;
                    }
                }

                if !player.flags.in_map_editor {
                    // block zones
                    valid &= check_collision_with(
                        player.position,
                        map.zones.iter().filter(|zone| zone.data == ZoneData::Blocked),
                        |zone| Box2D::from_origin_and_size(zone.position.into(), zone.size.into()),
                    )
                    .is_none();

                    valid &= check_collision_with(
                        player.position,
                        player_boxes.iter().filter(|(cid, _box2d)| cid != client_id),
                        |(_cid, box2d)| *box2d,
                    )
                    .is_none();
                }

                if valid {
                    player.position = new_position;
                }
            }

            if !player.flags.in_map_editor {
                let warp = check_collision_with(
                    player.position,
                    map.zones
                        .iter()
                        .filter(|zone| matches!(zone.data, ZoneData::Warp(_, _, _))),
                    |zone| Box2D::from_origin_and_size(zone.position.into(), zone.size.into()),
                );

                if let Some(Zone {
                    data: ZoneData::Warp(map_id, position, direction),
                    ..
                }) = warp
                {
                    to_warp.push((
                        *client_id,
                        map_id.clone(),
                        WarpParams {
                            position: Some((*position).into()),
                            direction: *direction,
                            ..Default::default()
                        },
                    ));
                }
            }
        }

        for (client_id, map_id, params) in to_warp {
            self.validate_map(&map_id);
            self.warp_player(client_id, &map_id, params);
        }
    }

    // Convenience function to validate that a map exists by it's name, and then return it's hash
    fn validate_map(&mut self, map_id: &str) {
        if let Entry::Vacant(e) = self.maps.entry(map_id.to_string()) {
            e.insert(create_map(map_id));
        }
    }

    /// Warps the player to a specific map, sending all the correct packets
    fn warp_player(&mut self, client_id: ClientId, map_id: &str, params: WarpParams) {
        if !self.players.contains_key(&client_id) {
            return;
        }

        let old_map = self.players[&client_id].map.as_ref();

        // check if we're actually changing maps, or if we're just moving to a new position.
        if params.initial || self.players[&client_id].map != map_id {
            if !params.initial {
                self.send_map_except(old_map, client_id, &Packet::RemoveData(client_id));
            }

            self.players.get_mut(&client_id).unwrap().map = map_id.to_string();
            let cache_key = self.maps[map_id].settings.cache_key;

            self.send(client_id, &Packet::ChangeMap(map_id.to_string(), cache_key));

            let packets = self
                .players
                .iter()
                .filter(|(_, player_data)| player_data.map == map_id)
                .map(|(&cid, data)| Packet::PlayerData(cid, data.clone().into()))
                .collect::<Vec<_>>();

            for packet in packets {
                self.send(client_id, &packet);
            }

            let player_data = self.players[&client_id].clone();
            self.send_to_map(map_id, &Packet::PlayerData(client_id, player_data.into()));
        }

        if let Some(player) = self.players.get_mut(&client_id) {
            if let Some(position) = params.position {
                player.position = position;
            }
            if let Some(direction) = params.direction {
                player.direction = direction;
                player.velocity = None;
            }

            let packet = Packet::PlayerMove {
                client_id,
                position: player.position.into(),
                direction: player.direction,
                velocity: player.velocity.map(Into::into),
            };

            player.save().unwrap();
            self.send_to_map(map_id, &packet);
        }
    }

    pub fn maintain(&mut self) {
        // lol
    }
}

/// Network convenience
impl GameServer {
    pub fn network(&self) -> &NetworkController {
        self.handler.as_ref().unwrap().network()
    }

    pub fn send(&self, client_id: ClientId, message: &Packet) {
        let endpoint = self.peer_map[&client_id];
        let bytes = rmp_serde::to_vec(&message).unwrap();
        self.network().send(endpoint, &bytes);
    }

    pub fn send_exclude(&self, exclude: ClientId, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for (&cid, &endpoint) in &self.peer_map {
            if cid != exclude {
                self.network().send(endpoint, &bytes);
            }
        }
    }

    pub fn send_list(&self, client_list: &[ClientId], message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for client_id in client_list {
            let endpoint = self.peer_map[client_id];
            self.network().send(endpoint, &bytes);
        }
    }

    pub fn send_to_map(&self, map_id: &str, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for (client_id, player) in &self.players {
            if player.map == map_id {
                let endpoint = self.peer_map[client_id];
                self.network().send(endpoint, &bytes);
            }
        }
    }

    pub fn send_map_except(&self, map_id: &str, exclude_id: ClientId, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for (client_id, player) in &self.players {
            if player.map == map_id && *client_id != exclude_id {
                let endpoint = self.peer_map[client_id];
                self.network().send(endpoint, &bytes);
            }
        }
    }

    pub fn send_all(&self, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for &endpoint in self.peer_map.values() {
            self.network().send(endpoint, &bytes);
        }
    }
}

fn check_bounds(position: Point2D<f32>, bounds: Box2D<f32>) -> Option<Direction> {
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
    position: Point2D<f32>,
    mut blockers: impl Iterator<Item = &'a T>,
    map_with: impl Fn(&'a T) -> Box2D<f32>,
) -> Option<&'a T> {
    let sprite = sprite_box(position);

    blockers.find(|item| sprite.intersects(&map_with(item)))
}

fn check_edge_warp(map: &Map, position: Point2D<f32>) -> Option<(Direction, Point2D<f32>)> {
    let map_box = map.to_box2d();

    if let Some(direction) = check_bounds(position, map_box) {
        let map_rect = map_box.to_rect();
        let new_position = match direction {
            Direction::North => Point2D::new(position.x, map_rect.max_y() - SPRITE_SIZE as f32),
            Direction::South => Point2D::new(position.x, -SPRITE_SIZE as f32 / 2.0),
            Direction::West => Point2D::new(map_rect.max_x() - SPRITE_SIZE as f32, position.y),
            Direction::East => Point2D::new(0.0, position.y),
        };

        Some((direction, new_position))
    } else {
        None
    }
}

fn create_map(id: &str) -> Map {
    let map = Map::new(id, 20, 15);
    map.save().unwrap();
    map
}

fn sprite_box(position: Point2D<f32>) -> Box2D<f32> {
    const SPRITE_SIZE2D: Size2D<f32> = Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0);
    const SPRITE_OFFSET: Vector2D<f32> = Vector2D::new(0.0, SPRITE_SIZE as f32 / 2.0);

    Box2D::from_origin_and_size(position + SPRITE_OFFSET, SPRITE_SIZE2D)
}

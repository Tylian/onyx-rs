mod data;
mod player;

use std::{
    collections::{hash_map::Entry, HashMap},
    time::{Duration, Instant},
};

use anyhow::Result;
use base64ct::{Base64, Encoding};
use chrono::Utc;
use common::{
    network::{ChatChannel, ClientId, ClientMessage, Direction, FailJoinReason, MapId, ServerMessage, ZoneData},
    SPRITE_SIZE,
};
use data::Player as PlayerData;
use data::{Config, Map};
use euclid::default::{Point2D, Size2D, Vector2D, Box2D};
use message_io::{
    network::{Endpoint, NetworkController, Transport},
    node::{self, NodeHandler, StoredNetEvent},
};
use player::Player;
use sha2::{Digest, Sha256};
use rand::prelude::*;

use crate::data::NameCache;

fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    #[cfg(not(debug_assertions))]
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let game_server = GameServer::new()?;
    game_server.run();

    Ok(())
}

#[derive(Default, Debug)]
struct WarpParams {
    initial: bool,
    position: Option<Point2D<f32>>,
    direction: Option<Direction>,
    velocity: Option<Option<Vector2D<f32>>>,
}

struct GameServer {
    config: Config,
    players: HashMap<ClientId, Player>,
    peer_map: HashMap<ClientId, Endpoint>,
    maps: HashMap<MapId, Map>,
    time: Instant,
    /// Time since last update
    dt: Duration,
    handler: Option<NodeHandler<()>>,
    rng: ThreadRng
}

impl GameServer {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let mut maps = Map::load_all()?;

        if let Entry::Vacant(e) = maps.entry(config.start.map) {
            e.insert(create_map(config.start.map));
        }

        Ok(Self {
            config,
            players: HashMap::new(),
            peer_map: HashMap::new(),
            time: Instant::now(),
            dt: Duration::ZERO,
            handler: None,
            maps,
            rng: rand::thread_rng()
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
                        let message = bincode::deserialize(&bytes).unwrap();
                        let client_id = peer_map
                            .get(&endpoint)
                            .expect("receiving from an endpoint that doesn't have an id??");
                        self.handle_login_message(*client_id, message);
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
                ServerMessage::PlayerLeft(client_id),
            );

            let goodbye = ServerMessage::Message(ChatChannel::Server, format!("{} has left the game.", &player.name));
            self.send_exclude(client_id, goodbye);

            PlayerData::from(player).save().unwrap();
        }
    }

    fn handle_login_message(&mut self, client_id: ClientId, message: ClientMessage) {
        use ClientMessage::*;

        match &message {
            SaveMap(_) => log::debug!("SaveMap(..)"),
            message => {
                log::debug!("{client_id:?}: {message:?}");
            }
        }

        if self.players.contains_key(&client_id) {
            return self.handle_message(client_id, message);
        }

        match message {
            CreateAccount {
                username,
                password,
                character_name: name,
            } => {
                let mut name_cache = NameCache::load().unwrap();

                if PlayerData::path(&username).exists() {
                    self.send(client_id, ServerMessage::FailedJoin(FailJoinReason::UsernameTaken));
                }

                if name_cache.contains(&name) {
                    self.send(client_id, ServerMessage::FailedJoin(FailJoinReason::CharacterNameTaken));
                }

                name_cache.insert(name.clone());
                name_cache.save().unwrap();

                let hash = Sha256::digest(password);
                let password = Base64::encode_string(&hash);

                let player = PlayerData {
                    username,
                    password,
                    name,
                    position: Point2D::new(self.config.start.x, self.config.start.y),
                    map: self.config.start.map,
                    ..Default::default()
                };

                player.save().unwrap();

                self.join_game(client_id, player.into());
            }
            Login { username, password } => {
                let incorrect = ServerMessage::FailedJoin(FailJoinReason::LoginIncorrect);

                if let Ok(player) = PlayerData::load(&username) {
                    let hash = Sha256::digest(password);
                    let password = Base64::encode_string(&hash);
                    if player.password == password {
                        self.join_game(client_id, player.into());
                    } else {
                        self.send(client_id, incorrect);
                    }
                } else {
                    self.send(client_id, incorrect);
                }
            }
            _ => {
                log::error!("Client sent a packet when it's not connected");
            }
        }
    }

    fn handle_message(&mut self, client_id: ClientId, message: ClientMessage) {
        use ClientMessage::*;

        match message {
            CreateAccount { .. } => unreachable!(),
            Login { .. } => unreachable!(),

            Message(channel, text) => {
                if let Some(player) = self.players.get(&client_id) {
                    match channel {
                        ChatChannel::Echo | ChatChannel::Error => log::warn!("Client tried to talk in an invalid channel"),
                        ChatChannel::Server => {
                            let packet = ServerMessage::Message(ChatChannel::Server, text);
                            self.send_all(packet);
                        }
                        ChatChannel::Say => {
                            let full_text = format!("{}: {}", player.name, text);

                            let packet = ServerMessage::Message(ChatChannel::Say, full_text);
                            self.send_map(player.map, packet);
                        }
                        ChatChannel::Global => {
                            let full_text = format!("{}: {}", player.name, text);
                            let packet = ServerMessage::Message(ChatChannel::Global, full_text);
                            self.send_all(packet);
                        }
                    }
                }
            }
            RequestMap => {
                if let Some(map_id) = self.players.get(&client_id).map(|p| p.map) {
                    let map = self.maps.entry(map_id).or_insert_with(|| create_map(map_id));

                    let packet = ServerMessage::MapData(Box::new(map.clone().into()));
                    self.send(client_id, packet);
                }
            }
            SaveMap(map) => {
                let map_id = self.players.get(&client_id).map(|p| p.map).unwrap();
                let mut map = Map::from(*map);

                map.settings.cache_key = Utc::now().timestamp_millis();
                
                if let Err(e) = map.save() {
                    log::error!("Couldn't save map {e}");
                }

                self.maps.insert(map_id, map.clone());

                self.send_map(map_id, ServerMessage::MapData(Box::new(map.into())));
            }
            Move {
                position,
                direction,
                velocity,
            } => {
                let player = self.players.get_mut(&client_id).unwrap();
                player.position = position.into();
                player.velocity = velocity.map(Into::into);
                player.direction = direction;

                let packet = ServerMessage::PlayerMove {
                    client_id,
                    position,
                    direction,
                    velocity,
                };

                let map_id = player.map;
                self.send_map(map_id, packet);
            }
            Warp(map_id, position) => {
                // note: the requested map possibly doesn't exist
                self.warp_player(
                    client_id,
                    map_id,
                    WarpParams {
                        position: position.map(Into::into),
                        velocity: None,
                        ..Default::default()
                    },
                );

                self.send_map_editor(client_id, map_id);
            }
            MapEditor(open) => {
                let player = self.players.get_mut(&client_id).unwrap();
                player.flags.in_map_editor = open;

                let map_id = player.map;
                let flags = player.flags;

                self.send_map(map_id, ServerMessage::Flags(client_id, flags));

                if open {
                    self.send_map_editor(client_id, map_id);
                }
            }
        }
    }

    fn send_map_editor(&self, client_id: ClientId, map_id: MapId) {
        let maps = self
            .maps
            .iter()
            .map(|(&id, map)| (id, map.settings.name.clone()))
            .collect::<HashMap<_, _>>();
        let map = self.maps.get(&map_id).unwrap();

        let id = map_id;
        let width = map.width;
        let height = map.height;
        let settings = map.settings.clone();

        self.send(
            client_id,
            ServerMessage::MapEditor {
                maps,
                id,
                width,
                height,
                settings,
            },
        );
    }

    fn join_game(&mut self, client_id: ClientId, player: Player) {
        // Save their data
        self.players.insert(client_id, player.clone());

        // Send them their ID
        self.send(client_id, ServerMessage::JoinGame(client_id));

        self.warp_player(
            client_id,
            player.map,
            WarpParams {
                initial: true,
                ..Default::default()
            },
        );

        // Send welcome message
        self.send(
            client_id,
            ServerMessage::Message(ChatChannel::Server, "Welcome to Game™!".to_owned()),
        );

        // Send join message
        self.send_exclude(
            client_id,
            ServerMessage::Message(ChatChannel::Server, format!("{} has joined the game.", &player.name)),
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

        let player_boxes = self.players.iter()
            .map(|(client_id, player)| (
                *client_id,
                Box2D::from_origin_and_size(
                    player.position + sprite_offset,
                    sprite_size
                )
            ))
            .collect::<Vec<_>>();

        for (client_id, player) in &mut self.players {
            let map = match self.maps.get(&player.map) {
                Some(map) => map,
                None => continue,
            };

            if let Some(velocity) = player.velocity {
                let offset = velocity * dt.as_secs_f32();
                let new_position = player.position + offset;
                let mut valid = true;

                // only block on the bottom half of the sprite, feels better
                let sprite = Box2D::from_origin_and_size(
                    new_position + sprite_offset, 
                    sprite_size
                );

                let map_box = map.to_box2d();

                if !player.flags.in_map_editor {
                    // map warps, lots of copy paste code lol
                    if let Some((map_id, new_position)) = check_edge_warp(map, new_position) {
                        to_warp.push((
                            *client_id,
                            map_id,
                            WarpParams {
                                position: Some(new_position),
                                ..Default::default()
                            },
                        ));
                        valid = false;
                    }
                }

                // map bounds
                valid &= map_box.contains_box(&sprite);

                if !player.flags.in_map_editor {
                    // block zones
                    valid &= !map.zones.iter()
                        .filter(|zone| zone.data == ZoneData::Blocked)
                        .map(|zone| Box2D::from_origin_and_size(zone.position.into(), zone.size.into()))
                        .any(|box2d| box2d.intersects(&sprite));

                    // other players
                    valid &= !player_boxes.iter()
                        .filter(|(cid, _box2d)| cid != client_id)
                        .any(|(_cid, box2d)| box2d.intersects(&sprite));
                }

                log::debug!("{valid}");

                if valid {
                    player.position = new_position;
                }
            }

            let sprite = Box2D::from_origin_and_size(
                Point2D::new(player.position.x, player.position.y + SPRITE_SIZE as f32 / 2.0),
                Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0),
            );

            for zone in map.zones.iter() {
                match &zone.data {
                    ZoneData::Warp(map_id, position, direction) => {
                        if !player.flags.in_map_editor {
                            let box2d = Box2D::from_origin_and_size(zone.position.into(), zone.size.into());
                            if box2d.intersects(&sprite) {
                                to_warp.push((
                                    *client_id,
                                    *map_id,
                                    WarpParams {
                                        position: Some((*position).into()),
                                        direction: *direction,
                                        velocity: direction.map(|_| None),
                                        ..Default::default()
                                    },
                                ));
                            }
                        }
                    }
                    ZoneData::Blocked => (),
                }
            }
        }

        for (client_id, map_id, params) in to_warp {
            self.warp_player(client_id, map_id, params);
        }
    }

    /// Warps the player to a specific map, sending all the correct packets
    fn warp_player(&mut self, client_id: ClientId, map_id: MapId, params: WarpParams) {
        log::debug!("{:?}, {:?}, {:?}", client_id, map_id, params);
        if !self.players.contains_key(&client_id) {
            return;
        }

        if let Entry::Vacant(e) = self.maps.entry(map_id) {
            e.insert(create_map(map_id));
        }

        let old_map = self.players.get(&client_id).unwrap().map;

        // check if we're actually changing maps, or if we're just moving to a new position.
        if params.initial || self.players.get(&client_id).unwrap().map != map_id {
            if !params.initial {
                self.send_map_except(old_map, client_id, ServerMessage::PlayerLeft(client_id));
            }

            self.players.get_mut(&client_id).unwrap().map = map_id;
            let cache_key = self.maps.get(&map_id).map(|m| m.settings.cache_key).unwrap_or(0);

            self.send(client_id, ServerMessage::ChangeMap(map_id, cache_key));

            let packets = self.players.iter()
                .filter(|(_, data)| data.map == map_id)
                .map(|(&cid, data)| ServerMessage::PlayerJoined(cid, data.clone().into()))
                .collect::<Vec<_>>();

            for packet in packets {
                self.send(client_id, packet);
            }

            let player_data = self.players.get(&client_id).unwrap().clone().into();
            self.send_map(map_id, ServerMessage::PlayerJoined(client_id, player_data));
        }

        if let Some(player) = self.players.get_mut(&client_id) {
            if let Some(v) = params.position {
                player.position = v;
            }
            if let Some(v) = params.direction {
                player.direction = v;
            }
            if let Some(v) = params.velocity {
                player.velocity = v;
            }

            let packet = ServerMessage::PlayerMove {
                client_id,
                position: player.position.into(),
                direction: player.direction,
                velocity: player.velocity.map(Into::into),
            };
            self.send_map(map_id, packet);
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

    pub fn send(&self, client_id: ClientId, message: ServerMessage) {
        if let Some(&endpoint) = self.peer_map.get(&client_id) {
            let bytes = bincode::serialize(&message).unwrap();
            self.network().send(endpoint, &bytes);
        }
    }

    pub fn send_exclude(&self, exclude: ClientId, message: ServerMessage) {
        let bytes = bincode::serialize(&message).unwrap();
        for (&cid, &endpoint) in self.peer_map.iter() {
            if cid != exclude {
                self.network().send(endpoint, &bytes);
            }
        }
    }

    pub fn send_list(&self, client_list: &[ClientId], message: ServerMessage) {
        let bytes = bincode::serialize(&message).unwrap();
        for client_id in client_list.iter() {
            if let Some(&endpoint) = self.peer_map.get(client_id) {
                self.network().send(endpoint, &bytes);
            }
        }
    }

    pub fn send_map(&self, map_id: MapId, message: ServerMessage) {
        let bytes = bincode::serialize(&message).unwrap();
        for (client_id, player) in self.players.iter() {
            if player.map == map_id {
                if let Some(&endpoint) = self.peer_map.get(client_id) {
                    self.network().send(endpoint, &bytes);
                }
            }
        }
    }

    pub fn send_map_except(&self, map_id: MapId, exclude_id: ClientId, message: ServerMessage) {
        let bytes = bincode::serialize(&message).unwrap();
        for (client_id, player) in self.players.iter() {
            if player.map == map_id && *client_id != exclude_id {
                if let Some(&endpoint) = self.peer_map.get(client_id) {
                    self.network().send(endpoint, &bytes);
                }
            }
        }
    }

    pub fn send_all(&self, message: ServerMessage) {
        let bytes = bincode::serialize(&message).unwrap();
        for &endpoint in self.peer_map.values() {
            self.network().send(endpoint, &bytes);
        }
    }
}

fn check_edge_warp(map: &Map, position: Point2D<f32>) -> Option<(MapId, Point2D<f32>)> {
    let sprite = sprite_box(position);

    let map_rect = map.to_box2d().to_rect();
    for (direction, warp) in map.settings.warps.iter() {
        if let Some(map_id) = warp {
            let valid = match direction {
                Direction::North => sprite.min.y <= map_rect.min_y(),
                Direction::South => sprite.max.y >= map_rect.max_y(),
                Direction::West => sprite.min.x <= map_rect.min_x(),
                Direction::East => sprite.max.x >= map_rect.max_x(),
            };

            if valid {
                let new_position = match direction {
                    Direction::North => Point2D::new(position.x, map_rect.max_y() - SPRITE_SIZE as f32),
                    Direction::South => Point2D::new(position.x, -SPRITE_SIZE as f32 / 2.0),
                    Direction::West => Point2D::new(map_rect.max_x() - SPRITE_SIZE as f32, position.y),
                    Direction::East => Point2D::new(0.0, position.y),
                };

                return Some((*map_id, new_position));
            }
        }
    }

    None
}

fn create_map(id: MapId) -> Map {
    let map = Map::new(id, 20, 15);
    map.save().unwrap();
    map
}

fn sprite_box(position: Point2D<f32>) -> Box2D<f32> {
    const SPRITE_SIZE2D: Size2D<f32>  = Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0);
    const SPRITE_OFFSET: Vector2D<f32> = Vector2D::new(0.0, SPRITE_SIZE as f32 / 2.0);

    Box2D::from_origin_and_size(
        position + SPRITE_OFFSET, 
        SPRITE_SIZE2D
    )
}
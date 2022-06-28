mod data;
mod player;

use std::{
    collections::{hash_map::Entry, HashMap},
    time::{Duration, Instant},
};

use anyhow::Result;
use base64ct::{Base64, Encoding};
use common::{
    network::{ZoneData, ChatMessage, ClientId, ClientMessage, Direction, FailJoinReason, MapId, ServerMessage},
    SPRITE_SIZE, TILE_SIZE,
};
use data::Player as PlayerData;
use data::{Config, Map};
use euclid::default::{Point2D, Rect, Size2D, Vector2D};
use message_io::{
    network::{Endpoint, NetworkController, Transport},
    node::{self, NodeHandler, StoredNetEvent},
};
use player::Player;
use sha2::{Digest, Sha256};

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

fn create_map(id: MapId) -> Map {
    let map = Map::new(id, 20, 15);
    map.save().unwrap();
    map
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

            let goodbye = ServerMessage::Message(ChatMessage::Server(format!("{} has left the game.", &player.name)));
            self.send_exclude(client_id, goodbye);

            PlayerData::from(player).save().unwrap();
        }
    }

    fn handle_login_message(&mut self, client_id: ClientId, message: ClientMessage) {
        use ClientMessage::*;

        log::debug!("{:?}: {:?}", client_id, message);

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

            Message(text) => {
                if let Some(player) = self.players.get(&client_id) {
                    let full_text = format!("{}: {}", player.name, text);
                    let packet = ServerMessage::Message(ChatMessage::Say(full_text));
                    self.send_all(packet);
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
                let map = Map::from(*map);

                if let Err(e) = map.save() {
                    log::error!("Couldn't save map {e}");
                }

                self.maps.insert(map_id, map.clone());

                self.send_list(
                    &self
                        .players
                        .iter()
                        .filter(|(_, data)| data.map == map_id)
                        .map(|(&cid, _)| cid)
                        .collect::<Vec<_>>(),
                    ServerMessage::MapData(Box::new(map.into())),
                );
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
                let players = self
                    .players
                    .iter()
                    .filter(|(_cid, data)| data.map == map_id)
                    .map(|(&cid, _)| cid)
                    .collect::<Vec<_>>();
                self.send_list(&players, packet);
            }
            Warp(map_id, position) => {
                self.warp_player(
                    client_id,
                    map_id,
                    WarpParams {
                        position: position.map(Into::into),
                        velocity: None,
                        ..Default::default()
                    },
                );
            }
            MapEditor => {
                let map_id = self.players.get(&client_id).map(|p| &p.map).unwrap();
                let maps = self
                    .maps
                    .iter()
                    .map(|(&id, map)| (id, map.settings.name.clone()))
                    .collect::<HashMap<_, _>>();
                let map = self.maps.get(map_id).unwrap();

                let id = *map_id;
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
        }
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
            ServerMessage::Message(ChatMessage::Server("Welcome to Game™!".to_owned())),
        );

        // Send join message
        self.send_exclude(
            client_id,
            ServerMessage::Message(ChatMessage::Server(format!("{} has joined the game.", &player.name))),
        );
    }

    fn tick(&mut self) {
        self.update_players();
    }

    fn update_players(&mut self) {
        let dt = self.dt;

        let mut to_warp = Vec::new();

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
                let sprite = Rect::new(
                    Point2D::new(new_position.x, new_position.y + SPRITE_SIZE as f32 / 2.0),
                    Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0),
                )
                .to_box2d();

                let (map_width, map_height) = (
                    map.width as f32 * TILE_SIZE as f32,
                    map.height as f32 * TILE_SIZE as f32,
                );

                // map warps, lots of copy paste code lol
                if let Some(map_id) = map.settings.warps.north {
                    if sprite.min.y <= 0.0 {
                        let height = self.maps.get(&map_id).unwrap().height as f32 * TILE_SIZE as f32;
                        to_warp.push((
                            *client_id,
                            map_id,
                            WarpParams {
                                position: Some(Point2D::new(new_position.x, height - SPRITE_SIZE as f32)),
                                ..Default::default()
                            },
                        ));
                        valid = false;
                    }
                }

                if let Some(map_id) = map.settings.warps.south {
                    if sprite.max.y >= map_height {
                        to_warp.push((
                            *client_id,
                            map_id,
                            WarpParams {
                                position: Some(Point2D::new(new_position.x, -SPRITE_SIZE as f32 / 2.0)),
                                ..Default::default()
                            },
                        ));
                        valid = false;
                    }
                }

                if let Some(map_id) = map.settings.warps.west {
                    if sprite.min.x <= 0.0 {
                        let width = self.maps.get(&map_id).unwrap().width as f32 * TILE_SIZE as f32;
                        to_warp.push((
                            *client_id,
                            map_id,
                            WarpParams {
                                position: Some(Point2D::new(width - SPRITE_SIZE as f32, new_position.y)),
                                ..Default::default()
                            },
                        ));
                        valid = false;
                    }
                }

                if let Some(map_id) = map.settings.warps.east {
                    if sprite.max.x >= map_width {
                        to_warp.push((
                            *client_id,
                            map_id,
                            WarpParams {
                                position: Some(Point2D::new(0.0, new_position.y)),
                                ..Default::default()
                            },
                        ));
                        valid = false;
                    }
                }

                // todo: method on map?
                let map_box = Rect::new(
                    Point2D::zero(),
                    Size2D::new(
                        map.width as f32 * TILE_SIZE as f32,
                        map.height as f32 * TILE_SIZE as f32,
                    ),
                )
                .to_box2d();

                valid &= map_box.contains_box(&sprite);
                valid &= !map.zones.iter().any(|attrib| {
                    let box2d = Rect::new(attrib.position.into(), attrib.size.into()).to_box2d();
                    attrib.data == ZoneData::Blocked && box2d.intersects(&sprite)
                });

                if valid {
                    player.position = new_position;
                }
            }

            let sprite = Rect::new(
                Point2D::new(player.position.x, player.position.y + SPRITE_SIZE as f32 / 2.0),
                Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0),
            )
            .to_box2d();

            for zone in map.zones.iter() {
                match &zone.data {
                    ZoneData::Warp(map_id, position, direction) => {
                        let box2d = Rect::new(zone.position.into(), zone.size.into()).to_box2d();
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
        println!("{:?}, {:?}, {:?}", client_id, map_id, params);
        if !self.players.contains_key(&client_id) {
            return;
        }

        // check if we're actually changing maps, or if we're just moving to a new position.
        if params.initial || self.players.get(&client_id).unwrap().map != map_id {
            if !params.initial {
                let list = self
                    .players
                    .iter()
                    .filter(|(&cid, data)| cid != client_id && data.map == map_id)
                    .map(|(&cid, _)| cid)
                    .collect::<Vec<_>>();

                self.send_list(&list, ServerMessage::PlayerLeft(client_id));
            }

            self.players.get_mut(&client_id).unwrap().map = map_id;
            let revision = self.maps.get(&map_id).map(|m| m.settings.revision).unwrap_or(0);

            self.send(client_id, ServerMessage::ChangeMap(map_id, revision));

            let packets = self
                .players
                .iter()
                .filter(|(_, data)| data.map == map_id)
                .map(|(&cid, data)| ServerMessage::PlayerJoined(cid, data.clone().into()))
                .collect::<Vec<_>>();

            for packet in packets {
                self.send(client_id, packet);
            }

            self.send_list(
                &self
                    .players
                    .iter()
                    .filter(|(_, data)| data.map == map_id)
                    .map(|(&cid, _)| cid)
                    .collect::<Vec<_>>(),
                ServerMessage::PlayerJoined(client_id, self.players.get(&client_id).unwrap().clone().into()),
            );
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

            println!("{packet:?}");

            self.send_list(
                &self
                    .players
                    .iter()
                    .filter(|(_, data)| data.map == map_id)
                    .map(|(&cid, _)| cid)
                    .collect::<Vec<_>>(),
                packet,
            );
        }
    }

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

    pub fn send_all(&self, message: ServerMessage) {
        let bytes = bincode::serialize(&message).unwrap();
        for &endpoint in self.peer_map.values() {
            self.network().send(endpoint, &bytes);
        }
    }

    pub fn maintain(&mut self) {
        // lol
    }
}

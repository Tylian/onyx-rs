use std::{sync::RwLock, collections::HashMap, fs, time::{Instant, Duration}};

use euclid::default::{Rect, Point2D, Vector2D, Size2D};
use anyhow::{anyhow, Result};
use onyx_common::{network::{PlayerData as NetworkPlayerData, ClientId, Map as NetworkMap, ServerMessage, ChatMessage, ClientMessage, AreaData}, SPRITE_SIZE, TILE_SIZE};

use crate::networking::{Networking, NetworkSignal, Message};

mod networking;

#[derive(Copy, Clone)]
pub struct Tween {
    pub velocity: Vector2D<f32>,
    pub last_update: Instant,
}

#[derive(Clone)]
struct PlayerData {
    name: String,
    sprite: u32,
    position: Point2D<f32>,
    tween: Option<Tween>,
    map: String,
    last_message: Instant
}

impl From<PlayerData> for NetworkPlayerData {
    fn from(other: PlayerData) -> Self {
        Self {
            name: other.name,
            sprite: other.sprite,
            position: other.position.into()
        }
    }
}

struct GameServer {
    network: RwLock<Networking>,
    network_queue: Vec<Message>,
    players: HashMap<ClientId, Option<PlayerData>>,
    maps: HashMap<String, NetworkMap>,
    time: Instant,
}

impl GameServer {
    pub fn new() -> Result<Self> {
        let mut network = Networking::new();
        network.listen("0.0.0.0:3042");

        let maps = Self::load_maps()?;

        Ok(Self {
            network: RwLock::new(network),
            network_queue: Vec::new(),
            players: HashMap::new(),
            time: Instant::now(),
            maps
        })
    }   

    pub fn run(self) {
        self.game_loop();
    }

    pub fn load_maps() -> Result<HashMap<String, NetworkMap>> {
        let mut maps = HashMap::new();
        for entry in fs::read_dir("./data/maps")? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let bytes = fs::read(&path)?;
                let map = bincode::deserialize(&bytes)?;

                let name = path.file_stem().ok_or_else(|| anyhow!("could not get file stem"))?;
                maps.insert(String::from(name.to_string_lossy()), map);
            }
        }

        // ensure there's a "start" map
        maps.entry("start".to_owned()).or_insert_with(|| NetworkMap::new(20, 15));
        Ok(maps)
    }

    pub fn save_map(&self, name: &str) -> anyhow::Result<()> {
        let map = self.maps.get(name).ok_or_else(|| anyhow!("map doesn't exist"))?;
        let bytes = bincode::serialize(&map)?;
        log::debug!("saving map {name}: {} bytes", bytes.len());
        fs::write(format!("./data/maps/{name}.bin"), bytes)?;
        Ok(())
    }

    fn handle_connect(&mut self, client_id: ClientId) {
        self.players.insert(client_id, None);
    }

    fn handle_disconnect(&mut self, client_id: ClientId) {
        self.queue(Message::everyone_except(client_id, ServerMessage::PlayerLeft(client_id)));
        if let Some(player) = self.players.remove(&client_id).flatten() {
            let goodbye = ServerMessage::Message(ChatMessage::Server(format!("{} has left the game.", &player.name)));
            self.queue(Message::everyone_except(client_id, goodbye));
        }
    }

    fn handle_message(&mut self, client_id: ClientId, message: ClientMessage) {
        log::debug!("{:?}: {:?}", client_id, message);
        match message {
            ClientMessage::Hello(name, sprite) => {
                let player = PlayerData {
                    name,
                    sprite,
                    position: Point2D::new(10. * 48., 7. * 48.),
                    map: String::from("start"),
                    tween: None,
                    last_message: self.time
                };

                // Send them their ID
                self.queue(Message::to(client_id, ServerMessage::Hello(client_id)));

                // Send them the map
                let map = self.maps
                    .entry(player.map.clone())
                    .or_insert_with(|| NetworkMap::new(20, 15))
                    .clone();
                self.queue(Message::to(client_id, ServerMessage::ChangeMap(map)));

                // Tell everyone else they joined
                self.queue(Message::everyone(ServerMessage::PlayerJoined(
                    client_id,
                    player.clone().into()
                )));
        
                // Send everyone else the fact that they joined
                let packets = self.players.iter()
                    .filter_map(|(k, v)| v.as_ref().map(|v| (k, v)))
                    .map(|(id, player)| Message::to(client_id,  ServerMessage::PlayerJoined(*id, player.clone().into())))
                    .collect::<Vec<_>>();
        
                self.queue_all(&packets);

                // Send welcome message
                self.queue(Message::to(client_id, ServerMessage::Message(ChatMessage::Server("Welcome to Game™!".to_owned()))));

                // Send join message
                let welcome = ServerMessage::Message(ChatMessage::Server(format!("{} has joined the game.", &player.name)));
                self.queue(Message::everyone_except(client_id, welcome));

                // Save their data, they are now officially in game
                self.players.insert(client_id, Some(player));
            },

            ClientMessage::Message(text) => {
                if let Some(player) = self.players.get(&client_id).unwrap() {
                    let full_text = format!("{}: {}", player.name, text);
                    let packet = ServerMessage::Message(ChatMessage::Say(full_text));
                    self.queue(Message::everyone(packet));
                }
            },
            ClientMessage::RequestMap => {
                if let Some(player) = self.players.get(&client_id).unwrap() {
                    let map = self.maps.entry(player.map.clone())
                        .or_insert_with(|| NetworkMap::new(20, 15));
                    let packet = ServerMessage::ChangeMap(map.clone());
                    self.queue(Message::to(client_id, packet));
                }
            },
            ClientMessage::SaveMap(remote) => {
                if let Some(player) = self.players.get(&client_id).unwrap() {
                    self.maps.insert(player.map.clone(), remote.clone());
                    if let Err(e) = self.save_map(&player.map) {
                        log::error!("Couldn't save map {e}");
                    }
                    let packet = ServerMessage::ChangeMap(remote);
                    self.queue(Message::everyone(packet));
                }
            },
            ClientMessage::Move { position, direction, velocity } => {
                if let Some(player) = self.players.get_mut(&client_id).unwrap() {
                    player.position = position.into();
                    player.tween = velocity.map(|v| Tween { velocity: v.into(), last_update: self.time });
                    let packet = ServerMessage::PlayerMoved { client_id, position, direction, velocity };
                    self.queue(Message::everyone_except(client_id, packet));
                }
            },
        }
    }

    fn game_loop(mut self) {
        loop {
            self.time = Instant::now();

            // networking
            while let Some(signal) = self.try_recv() {
                match signal {
                    NetworkSignal::Message(client_id, message) => self.handle_message(client_id, message),
                    NetworkSignal::Connected(client_id) => self.handle_connect(client_id),
                    NetworkSignal::Disconnected(client_id) => self.handle_disconnect(client_id),
                }
            }

            // game loop
            self.update_players();
            
            // finalizing
            self.send_all();
            std::thread::yield_now();
        }
    }

    fn update_players(&mut self) {
        let mut packets = Vec::new();

        for (id, player) in &mut self.players {
            let player = match player {
                Some(player) => player,
                None => continue,
            };

            let map = match self.maps.get(&player.map) {
                Some(map) => map,
                None => continue,
            };

            if let Some(tween) = player.tween.as_mut() {
                let offset = tween.velocity * (self.time - tween.last_update).as_secs_f32();
                let new_position = player.position + offset;

                // only block on the bottom half of the sprite, feels better
                let sprite = Rect::new(
                    Point2D::new(new_position.x, new_position.y + SPRITE_SIZE as f32 / 2.0),
                    Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0)
                ).to_box2d();

                let map_size = Size2D::new(map.width as f32 * TILE_SIZE as f32, map.height as f32 * TILE_SIZE as f32); // todo map method
                let map_box = Rect::new(Point2D::zero(), map_size).to_box2d();

                let valid = map_box.contains_box(&sprite)
                    && !map.areas.iter().any(|attrib| {
                        let box2d = Rect::new(attrib.position.into(), attrib.size.into()).to_box2d();
                        attrib.data == AreaData::Blocked && box2d.intersects(&sprite)
                    });

                if valid {
                    player.position = new_position;
                }

                // ? need to update anyway even if we don't change anything
                // ? if we don't you can clip through stuff by walking against it for awhile
                tween.last_update = self.time;
            }

            let sprite = Rect::new(
                Point2D::new(player.position.x, player.position.y + SPRITE_SIZE as f32 / 2.0),
                Size2D::new(SPRITE_SIZE as f32, SPRITE_SIZE as f32 / 2.0)
            ).to_box2d();

            for attrib in map.areas.iter() {
                match &attrib.data {
                    AreaData::Log(message) => {
                        let box2d = Rect::new(attrib.position.into(), attrib.size.into()).to_box2d();
                        if box2d.intersects(&sprite) && player.last_message.elapsed() > Duration::from_secs(1) {
                            let message = ChatMessage::Server(message.clone());
                            packets.push(Message::to(*id, ServerMessage::Message(message)));
                            player.last_message = self.time;
                        }
                    },
                    AreaData::Blocked => (),
                }
            }
        }

        self.queue_all(&packets);
    }

    // Specifically created to avoid scope issues
    fn try_recv(&self) -> Option<NetworkSignal> {
        self.network.read().unwrap().try_recv()
    }

    pub fn queue(&mut self, message: Message) {
        self.network_queue.push(message);
    }

    pub fn queue_all(&mut self, messages: &[Message]) {
        self.network_queue.extend_from_slice(messages);
    }

    pub fn send_all(&mut self) {
        let network = self.network.read().unwrap();
        for message in self.network_queue.drain(..) {
            message.write(&network);
        }
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    #[cfg(not(debug_assertions))]
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let game_server = GameServer::new()?;
    game_server.run();

    Ok(())
}
use std::{sync::RwLock, collections::HashMap, fs, time};

use glam::*;
use log::{debug, error};
use anyhow::{anyhow, Result};
use onyx_common::network::{PlayerData as NetworkPlayerData, ClientId, Map as NetworkMap, ServerMessage, ChatMessage, ClientMessage};

use crate::networking::{Networking, NetworkSignal, Message};

mod networking;

#[derive(Copy, Clone)]
pub struct Tween {
    pub velocity: Vec2,
    pub last_update: time::Duration,
}

#[derive(Clone)]
struct PlayerData {
    name: String,
    sprite: u32,
    position: Vec2,
    tween: Option<Tween>,
    map: String
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
    data: HashMap<ClientId, Option<PlayerData>>,
    maps: HashMap<String, NetworkMap>,
    start_time: time::Instant,
    time: time::Duration,
}

impl GameServer {
    pub fn new() -> Result<Self> {
        let mut network = Networking::new();
        network.listen("0.0.0.0:3042");

        let maps = Self::load_maps()?;

        Ok(Self {
            network: RwLock::new(network),
            network_queue: Vec::new(),
            data: HashMap::new(),
            start_time: time::Instant::now(),
            time: time::Duration::ZERO,
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
        debug!("saving map {name}: {} bytes", bytes.len());
        fs::write(format!("./data/maps/{name}.bin"), bytes)?;
        Ok(())
    }

    fn handle_connect(&mut self, client_id: ClientId) {
        self.data.insert(client_id, None);
    }

    fn handle_disconnect(&mut self, client_id: ClientId) {
        self.queue(Message::everyone_except(client_id, ServerMessage::PlayerLeft(client_id)));
        if let Some(client_data) = self.data.remove(&client_id).flatten() {
            let goodbye = ServerMessage::Message(ChatMessage::Server(format!("{} has left the game.", &client_data.name)));
            self.queue(Message::everyone_except(client_id, goodbye));
        }
    }

    fn handle_message(&mut self, client_id: ClientId, message: ClientMessage) {
        debug!("{:?}: {:?}", client_id, message);
        match message {
            ClientMessage::Hello(name, sprite) => {
                let client_data = PlayerData {
                    name,
                    sprite,
                    position: glam::vec2(10. * 48., 7. * 48.),
                    map: String::from("start"),
                    tween: None,
                };

                // Send them their ID
                self.queue(Message::to(client_id, ServerMessage::Hello(client_id)));

                // Send them the map
                let map = self.maps
                    .entry(client_data.map.clone())
                    .or_insert_with(|| NetworkMap::new(20, 15))
                    .clone();
                self.queue(Message::to(client_id, ServerMessage::ChangeMap(map)));

                // Tell everyone else they joined
                self.queue(Message::everyone(ServerMessage::PlayerJoined(
                    client_id,
                    client_data.clone().into()
                )));
        
                // Send everyone else the fact that they joined
                let packets = self.data.iter()
                    .filter_map(|(k, v)| v.as_ref().map(|v| (k, v)))
                    .map(|(id, data)| Message::to(client_id,  ServerMessage::PlayerJoined(*id, data.clone().into())))
                    .collect::<Vec<_>>();
        
                self.queue_all(&packets);

                // Send welcome message
                self.queue(Message::to(client_id, ServerMessage::Message(ChatMessage::Server("Welcome to Game™!".to_owned()))));

                // Send join message
                let welcome = ServerMessage::Message(ChatMessage::Server(format!("{} has joined the game.", &client_data.name)));
                self.queue(Message::everyone_except(client_id, welcome));

                // Save their data, they are now officially in game
                self.data.insert(client_id, Some(client_data));
            },

            ClientMessage::Message(text) => {
                if let Some(data) = self.data.get(&client_id).unwrap() {
                    let full_text = format!("{}: {}", data.name, text);
                    let packet = ServerMessage::Message(ChatMessage::Say(full_text));
                    self.queue(Message::everyone(packet));
                }
            },
            ClientMessage::ChangeTile { position, layer, tile, is_autotile } => {
                let packet = ServerMessage::ChangeTile { position, layer, tile, is_autotile };
                self.queue(Message::everyone(packet));
            },
            ClientMessage::RequestMap => {
                if let Some(data) = self.data.get(&client_id).unwrap() {
                    let map = self.maps.entry(data.map.clone())
                        .or_insert_with(|| NetworkMap::new(20, 15));
                    let packet = ServerMessage::ChangeMap(map.clone());
                    self.queue(Message::to(client_id, packet));
                }
            },
            ClientMessage::SaveMap(remote) => {
                if let Some(data) = self.data.get(&client_id).unwrap() {
                    self.maps.insert(data.map.clone(), remote.clone());
                    if let Err(e) = self.save_map(&data.map) {
                        error!("Couldn't save map {e}");
                    }
                    let packet = ServerMessage::ChangeMap(remote);
                    self.queue(Message::everyone(packet));
                }
            },
            ClientMessage::Move { position, direction, velocity } => {
                if let Some(data) = self.data.get_mut(&client_id).unwrap() {
                    data.position = position.into();
                    data.tween = velocity.map(|v| Tween { velocity: v.into(), last_update: self.time });
                    let packet = ServerMessage::PlayerMoved { client_id, position, direction, velocity };
                    self.queue(Message::everyone_except(client_id, packet));
                }
            },
        }
    }

    fn game_loop(mut self) {
        loop {
            self.time = self.start_time.elapsed();

            // networking
            while let Some(signal) = self.try_recv() {
                match signal {
                    NetworkSignal::Message(client_id, message) => self.handle_message(client_id, message),
                    NetworkSignal::Connected(client_id) => self.handle_connect(client_id),
                    NetworkSignal::Disconnected(client_id) => self.handle_disconnect(client_id),
                }
            }

            // game loop
            // for (endpoint, client) in &mut self.data {
                
            // }
            
            // finalizing
            self.send_all();
            std::thread::yield_now();
        }
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
    let env = env_logger::Env::default()
        .filter_or(env_logger::DEFAULT_FILTER_ENV, if cfg!(debug_assertions) { "debug" } else { "info" });
    env_logger::init_from_env(env);

    let game_server = GameServer::new()?;
    game_server.run();

    Ok(())
}
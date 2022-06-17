use std::{sync::RwLock, collections::HashMap};

use common::network::*;
use glam::*;

use crate::networking::{Networking, NetworkSignal, Message};

mod networking;

#[derive(Clone)]
struct ClientData {
    name: String,
    sprite: u32,
    position: IVec2
}

impl From<ClientData> for PlayerData {
    fn from(other: ClientData) -> Self {
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
    data: HashMap<ClientId, Option<ClientData>>,
    map: RemoteMap,
}

impl GameServer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut network = Networking::new();
        network.listen("0.0.0.0:3042");

        let size = 20 * 15;

        Ok(Self {
            network: RwLock::new(network),
            network_queue: Vec::new(),
            data: HashMap::new(),
            map: RemoteMap {
                width: 20,
                height: 15,
                ground: vec![Default::default(); size],
                mask: vec![Default::default(); size],
                fringe: vec![Default::default(); size],
                attribute: vec![Default::default(); size],
            }
        })
    }   

    pub fn run(self) {
        self.game_loop();
    }

    fn handle_connect(&mut self, client_id: ClientId) {
        self.data.insert(client_id, None);
    }

    fn handle_disconnect(&mut self, client_id: ClientId) {
        self.queue(Message::send_to_all_but(client_id, ServerMessage::PlayerLeft(client_id)));
        if let Some(client_data) = self.data.remove(&client_id).flatten() {
            let goodbye = ServerMessage::Message(ChatMessage::Server(format!("{} has left the game.", &client_data.name)));
            self.queue(Message::send_to_all_but(client_id, goodbye));
        }
    }

    fn handle_message(&mut self, client_id: ClientId, message: ClientMessage) {
        println!("{:?}: {:?}", client_id, message);
        match message {
            ClientMessage::Hello(name, sprite) => {
                let client_data = ClientData {
                    name,
                    sprite,
                    position: glam::ivec2(10, 7),
                };

                // Send them their ID
                self.queue(Message::send_to(client_id, ServerMessage::Hello(client_id)));

                // Send them the map
                let packet = ServerMessage::ChangeMap(self.map.clone());
                self.queue(Message::send_to(client_id, packet));

                // Tell everyone else they joined
                self.queue(Message::send_to_all(ServerMessage::PlayerJoined(
                    client_id,
                    client_data.clone().into()
                )));
        
                // Send everyone else the fact that they joined
                let packets = self.data.iter()
                    .filter_map(|(k, v)| v.as_ref().map(|v| (k, v)))
                    .map(|(client_id, data)|
                        ServerMessage::PlayerJoined(*client_id, data.clone().into())
                    )
                    .collect::<Vec<_>>();
        
                for packet in packets {
                    self.queue(Message::send_to(client_id, packet));
                }

                // Send welcome message
                self.queue(Message::send_to(client_id, ServerMessage::Message(ChatMessage::Server("Welcome to Game!".to_owned()))));

                // Send join message
                let welcome = ServerMessage::Message(ChatMessage::Server(format!("{} has joined the game.", &client_data.name)));
                self.queue(Message::send_to_all_but(client_id, welcome));

                // Save their data, they are now officially in game
                self.data.insert(client_id, Some(client_data));
            },
            ClientMessage::Move(direction) => {
                if let Some(data) = self.data.get_mut(&client_id).unwrap().as_mut() {
                    data.position += direction.offset().into();
                    let message = ServerMessage::PlayerMoved { client_id, position: data.position.into(), direction };
                    self.queue(Message::send_to_all_but(client_id, message));
                }
            },
            ClientMessage::Message(text) => {
                if let Some(data) = self.data.get(&client_id).unwrap() {
                    let full_text = format!("{}: {}", data.name, text);
                    let packet = ServerMessage::Message(ChatMessage::Say(full_text));
                    self.queue(Message::send_to_all(packet));
                }
            },
            ClientMessage::ChangeTile { position, layer, tile, is_autotile } => {
                let packet = ServerMessage::ChangeTile { position, layer, tile, is_autotile };
                self.queue(Message::send_to_all(packet));
            },
            ClientMessage::RequestMap => {
                let packet = ServerMessage::ChangeMap(self.map.clone());
                self.queue(Message::send_to(client_id, packet));
            },
            ClientMessage::SaveMap(remote) => {
                self.map = remote;
                let packet = ServerMessage::ChangeMap(self.map.clone());
                self.queue(Message::send_to_all(packet));
            },
        }
    }

    fn game_loop(mut self) {
        loop {
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

    pub fn send_all(&mut self) {
        let network = self.network.read().unwrap();
        for message in self.network_queue.drain(..) {
            message.write(&network);
        }
    }
}

fn main() {
    let game_server = GameServer::new().unwrap();
    game_server.run();
}
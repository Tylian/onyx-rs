use std::{sync::RwLock, collections::HashMap};

use common::network::*;
use glam::Vec2;

use crate::networking::{Networking, NetworkSignal, Message};

mod networking;

#[derive(Clone)]
struct ClientData {
    name: String,
    sprite: u32,
    position: Vec2
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
    data: HashMap<ClientId, Option<ClientData>>
}

impl GameServer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut network = Networking::new();
        network.listen("0.0.0.0:3042");

        Ok(Self {
            network: RwLock::new(network),
            network_queue: Vec::new(),
            data: HashMap::new()
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
            let goodbye = ServerMessage::Message(ChatChannel::Server, format!("{} has left the game.", &client_data.name));
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
                    position: glam::vec2(10.0, 7.0),
                };

                self.queue(Message::send_to(client_id, ServerMessage::Hello(client_id)));
                self.queue(Message::send_to_all(ServerMessage::PlayerJoined(
                    client_id,
                    client_data.clone().into()
                )));
        
                let packets = self.data.iter()
                    .filter_map(|(k, v)| v.as_ref().map(|v| (k, v)))
                    .map(|(client_id, data)|
                        ServerMessage::PlayerJoined(*client_id, data.clone().into())
                    )
                    .collect::<Vec<_>>();
        
                for packet in packets {
                    self.queue(Message::send_to(client_id, packet));
                }

                self.queue(Message::send_to(client_id, ServerMessage::Message(ChatChannel::Server, "Welcome to Game!".to_owned())));

                let welcome = ServerMessage::Message(ChatChannel::Server, format!("{} has joined the game.", &client_data.name));
                self.queue(Message::send_to_all_but(client_id, welcome));

                self.data.insert(client_id, Some(client_data));
            },
            ClientMessage::Move(target, direction) => {
                if let Some(data) = self.data.get_mut(&client_id).unwrap().as_mut() {
                    let message = ServerMessage::PlayerMoved(client_id, data.position.into(), target, direction);
                    data.position = target.into();
                    self.queue(Message::send_to_all_but(client_id, message));
                }
            },
            ClientMessage::Message(text) => {
                if let Some(data) = self.data.get(&client_id).unwrap() {
                    let full_text = format!("{}: {}", data.name, text);
                    let packet = ServerMessage::Message(ChatChannel::Say, full_text);
                    self.queue(Message::send_to_all(packet));
                }
            },
            ClientMessage::ChangeTile(pos, index) => {
                let packet = ServerMessage::ChangeTile(pos, index);
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
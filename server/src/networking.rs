use std::{sync::mpsc::{self, Receiver, Sender}};
use std::thread;

use bimap::BiHashMap;
use common::network::*;
use message_io::network::{Transport, ToRemoteAddr, Endpoint};
use message_io::node::{self, StoredNetEvent};

// Represents a signal being sent from the game engine to the network layer
pub enum GameSignal {
    Send(MessageQualifier, ServerMessage),
    Disconnect
}

pub enum MessageQualifier {
    SendTo(ClientId),
    SendToAll,
    SendToAllBut(ClientId)
}

pub struct Message {
    qualifier: MessageQualifier,
    message: ServerMessage
}

impl Message {
    pub fn send_to(client_id: ClientId, message: ServerMessage) -> Self {
        Self {
            qualifier: MessageQualifier::SendTo(client_id),
            message
        }
    }
    pub fn send_to_all(message: ServerMessage) -> Self {
        Self {
            qualifier: MessageQualifier::SendToAll,
            message
        }
    }
    pub fn send_to_all_but(client_id: ClientId, message: ServerMessage) -> Self {
        Self {
            qualifier: MessageQualifier::SendToAllBut(client_id),
            message
        }
    }
    pub fn write(self, networking: &Networking) {
        networking.send(self.qualifier, self.message);
    }
}

// Represents a signal being sent from the network layer to the game engine
pub enum NetworkSignal {
    Message(ClientId, ClientMessage),
    Connected(ClientId),
    Disconnected(ClientId)
}

pub struct Networking { 
    game: (Sender<GameSignal>, Receiver<NetworkSignal>),
    network: Option<(Sender<NetworkSignal>, Receiver<GameSignal>)>,
}

impl Networking {
    pub fn new() -> Self {
        let (tx1, rx1) = mpsc::channel::<GameSignal>();  // game -> networking
        let (tx2, rx2) = mpsc::channel::<NetworkSignal>();  // game <- networking

        Self {
            game: (tx1, rx2),
            network: Some((tx2, rx1)),
        }
    }

    pub fn listen(&mut self, addr: impl ToRemoteAddr) {
        let (tx, rx) = self.network.take().unwrap();
        let addr = addr.to_remote_addr().unwrap();

        thread::spawn(move || {
            let (handler, listener) = node::split::<()>();
            handler.network()
                .listen(Transport::FramedTcp, addr.clone()).unwrap();
            
            println!("Listening on {}", addr);

            let (_task, mut receive) = listener.enqueue();

            let mut clients: BiHashMap<Endpoint, ClientId> = BiHashMap::new();
            let mut idx = 0u64;

            'network: loop {
                for signal in rx.try_iter() {
                    match signal {
                        GameSignal::Send(qualifier, data) => {
                            let bytes = bincode::serialize(&data).unwrap();
                            match qualifier {
                                MessageQualifier::SendTo(client_id) => {
                                    if let Some(endpoint) = clients.get_by_right(&client_id) {
                                        handler.network().send(*endpoint, &bytes);
                                    }
                                },
                                MessageQualifier::SendToAll => {
                                    for endpoint in clients.left_values() {
                                        handler.network().send(*endpoint, &bytes);
                                    }
                                },
                                MessageQualifier::SendToAllBut(exclude) => {
                                    for (endpoint, client_id) in clients.iter()  {
                                        if *client_id != exclude {
                                            handler.network().send(*endpoint, &bytes);
                                        }
                                    }
                                }
                            }
                            
                        },
                        GameSignal::Disconnect => {
                            handler.stop();
                            break 'network;
                        }
                    }
                }

                if let Some(event) = receive.try_receive() {
                    match event.network() {
                        StoredNetEvent::Connected(_, _) => unreachable!(),
                        StoredNetEvent::Accepted(endpoint, _listener) => {
                            idx += 1;
                            let client_id = ClientId::from(idx);
                            clients.insert(endpoint, client_id);
                            let signal = NetworkSignal::Connected(client_id);
                            tx.send(signal).unwrap();
                            println!("Client ({}) connected (total clients: {})", endpoint.addr(), clients.len());
                        },
                        StoredNetEvent::Message(endpoint, bytes) => {
                            let message = bincode::deserialize(&bytes).unwrap();
                            let client_id = clients.get_by_left(&endpoint).expect("receiving from an endpoint that doesn't have an id??");
                            let signal = NetworkSignal::Message(*client_id, message);
                            tx.send(signal).unwrap();
                        },
                        StoredNetEvent::Disconnected(endpoint) => {
                            let client_id = clients.get_by_left(&endpoint).expect("receiving from an endpoint that doesn't have an id??");
                            let signal = NetworkSignal::Disconnected(*client_id);
                            tx.send(signal).unwrap();
                            clients.remove_by_left(&endpoint);
                            println!("Client ({}) disconnected (total clients: {})", endpoint.addr(), clients.len())
                        }
                    }
                }
            }
        });
    }

    pub fn send(&self, qualifier: MessageQualifier, message: ServerMessage) {
        let (tx, _rx) = &self.game;
        let signal = GameSignal::Send(qualifier, message);
        tx.send(signal).unwrap();
    }

    pub fn try_recv(&self) -> Option<NetworkSignal> {
        let (_tx, rx) = &self.game;
        rx.try_recv().ok()
    }
    
    #[allow(dead_code)]
    pub fn disconnect(&self) {
        let (tx, _rx) = &self.game;
        tx.send(GameSignal::Disconnect).unwrap();
    }
}

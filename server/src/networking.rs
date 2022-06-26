use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use bimap::BiHashMap;
use common::network::*;
use log::info;
use message_io::network::{Endpoint, ToRemoteAddr, Transport};
use message_io::node::{self, StoredNetEvent};

// Represents a signal being sent from the game engine to the network layer
pub enum GameSignal {
    Send(MessageTarget, ServerMessage),
    Disconnect,
}

#[derive(Clone)]
pub enum MessageTarget {
    Only(ClientId),
    Exclude(ClientId),
    List(Vec<ClientId>),
    Nobody,
    Everybody,
}

#[derive(Clone)]
pub struct Message {
    target: MessageTarget,
    message: ServerMessage,
}

impl Message {
    pub fn only(client_id: ClientId, message: ServerMessage) -> Self {
        Self {
            target: MessageTarget::Only(client_id),
            message,
        }
    }
    pub fn everybody(message: ServerMessage) -> Self {
        Self {
            target: MessageTarget::Everybody,
            message,
        }
    }
    pub fn list(list: Vec<ClientId>, message: ServerMessage) -> Self {
        Self {
            target: MessageTarget::List(list),
            message,
        }
    }
    pub fn exclude(client_id: ClientId, message: ServerMessage) -> Self {
        Self {
            target: MessageTarget::Exclude(client_id),
            message,
        }
    }
    pub fn nobody(message: ServerMessage) -> Self {
        Self {
            target: MessageTarget::Nobody,
            message,
        }
    }

    pub fn write(self, networking: &Networking) {
        networking.send(self.target, self.message);
    }
}

// Represents a signal being sent from the network layer to the game engine
pub enum NetworkSignal {
    Message(ClientId, ClientMessage),
    Connected(ClientId),
    Disconnected(ClientId),
}

pub struct Networking {
    game: (Sender<GameSignal>, Receiver<NetworkSignal>),
    network: Option<(Sender<NetworkSignal>, Receiver<GameSignal>)>,
}

impl Networking {
    pub fn new() -> Self {
        let (tx1, rx1) = mpsc::channel::<GameSignal>(); // game -> networking
        let (tx2, rx2) = mpsc::channel::<NetworkSignal>(); // game <- networking

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
            handler.network().listen(Transport::FramedTcp, addr.clone()).unwrap();

            info!("Listening on {}", addr);

            let (_task, mut receive) = listener.enqueue();

            let mut clients: BiHashMap<Endpoint, ClientId> = BiHashMap::new();
            let mut idx = 0u64;

            'network: loop {
                for signal in rx.try_iter() {
                    match signal {
                        GameSignal::Send(target, data) => {
                            let bytes = bincode::serialize(&data).unwrap();
                            match target {
                                MessageTarget::Only(cid) => {
                                    if let Some(&endpoint) = clients.get_by_right(&cid) {
                                        handler.network().send(endpoint, &bytes);
                                    }
                                }
                                MessageTarget::Exclude(exclude) => {
                                    for (&endpoint, &cid) in clients.iter() {
                                        if cid != exclude {
                                            handler.network().send(endpoint, &bytes);
                                        }
                                    }
                                }
                                MessageTarget::List(list) => {
                                    for (&endpoint, &cid) in clients.iter() {
                                        if list.contains(&cid) {
                                            handler.network().send(endpoint, &bytes);
                                        }
                                    }
                                }
                                MessageTarget::Nobody => (),
                                MessageTarget::Everybody => {
                                    for &endpoint in clients.left_values() {
                                        handler.network().send(endpoint, &bytes);
                                    }
                                }
                            }
                        }
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
                            info!(
                                "Client ({}) connected (total clients: {})",
                                endpoint.addr(),
                                clients.len()
                            );
                        }
                        StoredNetEvent::Message(endpoint, bytes) => {
                            let message = bincode::deserialize(&bytes).unwrap();
                            let client_id = clients
                                .get_by_left(&endpoint)
                                .expect("receiving from an endpoint that doesn't have an id??");
                            let signal = NetworkSignal::Message(*client_id, message);
                            tx.send(signal).unwrap();
                        }
                        StoredNetEvent::Disconnected(endpoint) => {
                            let client_id = clients
                                .get_by_left(&endpoint)
                                .expect("receiving from an endpoint that doesn't have an id??");
                            let signal = NetworkSignal::Disconnected(*client_id);
                            tx.send(signal).unwrap();
                            clients.remove_by_left(&endpoint);
                            info!(
                                "Client ({}) disconnected (total clients: {})",
                                endpoint.addr(),
                                clients.len()
                            );
                        }
                    }
                }
            }
        });
    }

    pub fn send(&self, qualifier: MessageTarget, message: ServerMessage) {
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

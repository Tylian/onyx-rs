use std::{collections::HashSet, net::{SocketAddr, UdpSocket}, time::SystemTime};

use bimap::BiMap;
use common::network::{server::Packet, Entity};
use renet::{Bytes, ClientId, ConnectionConfig, DefaultChannel, RenetServer};
use renet::transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

use crate::data::Config;

pub struct Network {
    pub server: RenetServer,
    pub transport: NetcodeServerTransport,
    pub peer_map: BiMap<Entity, ClientId>,
    pub client_ids: HashSet<ClientId>,
    pub next_idx: u64,
}

impl Network {
    pub fn listen(config: &Config) -> Self {
        let server = RenetServer::new(ConnectionConfig::default());

        // Setup transport layer
        let server_addr: SocketAddr = config.listen.parse().unwrap();
        let socket: UdpSocket = UdpSocket::bind(server_addr).unwrap();
        let server_config = ServerConfig {
            current_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(),
            max_clients: 64,
            protocol_id: 0,
            public_addresses: vec![server_addr],
            authentication: ServerAuthentication::Unsecure
        };

        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

        log::info!("Listening on {}", server_addr);

        Self {
            server,
            transport,
            peer_map: BiMap::new(),
            client_ids: HashSet::new(),
            next_idx: 0,
        }
    }
}

impl Network {
    // pub fn network(&self) -> &NetworkController {
    //     self.handler.as_ref().unwrap().network()
    // }

    // #[inline]
    // fn send_bytes(&mut self, entity: Entity, bytes: Bytes) {
    //     if let Some(client_id) = self.peer_map.get_by_left(&entity) {
    //         self.server.send_message(*client_id, DefaultChannel::ReliableUnordered, bytes);
    //     }
    // }

    pub fn send_to(&mut self, client_id: ClientId, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        self.server.send_message(client_id, DefaultChannel::ReliableUnordered, bytes);
    }

    pub fn send(&mut self, entity: Entity, message: &Packet) {
        if let Some(client_id) = self.peer_map.get_by_left(&entity) {
            let bytes = rmp_serde::to_vec(&message).unwrap();
            self.server.send_message(*client_id, DefaultChannel::ReliableUnordered, bytes);
        }
    }

    pub fn send_list(&mut self, entity_list: &[Entity], message: &Packet) {
        let bytes = Bytes::from(rmp_serde::to_vec(&message).unwrap());
        for entity in entity_list {
            if let Some(client_id) = self.peer_map.get_by_left(entity) {
                self.server.send_message(*client_id, DefaultChannel::ReliableUnordered, bytes.clone());
            }
        }
    }

    pub fn broadcast(&mut self, message: &Packet) {
        let bytes = Bytes::from(rmp_serde::to_vec(&message).unwrap());
        for &client_id in self.peer_map.right_values() {
            self.server.send_message(client_id, DefaultChannel::ReliableUnordered, bytes.clone());
        }
    }

    pub fn broadcast_except(&mut self, exclude: Entity, message: &Packet) {
        let bytes: Bytes = Bytes::from(rmp_serde::to_vec(&message).unwrap());
        for (&entity, &client_id) in &self.peer_map {
            if entity == exclude {
                continue;
            }
            
            self.server.send_message(client_id, DefaultChannel::ReliableUnordered, bytes.clone());
        }
    }
}
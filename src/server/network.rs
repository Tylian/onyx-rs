use std::collections::HashSet;
use std::net::SocketAddr;

use bimap::BiMap;
use message_io::events::EventReceiver;
use message_io::node::{self, NodeHandler, NodeTask, StoredNodeEvent};
use message_io::network::{Endpoint, Transport};
use onyx::network::{server::Packet, Entity};
use thiserror::Error;

use crate::data::Config;

pub struct Network {
    pub handler: NodeHandler<()>,
    pub receiver: EventReceiver<StoredNodeEvent<()>>,
    pub peer_map: BiMap<Entity, Endpoint>,
    pub endpoints: HashSet<Endpoint>,
    
    #[allow(dead_code)] // RAII
    task: NodeTask,
}

#[derive(Clone, Copy, Debug, Error)]
pub enum NetworkError {
    #[error("could not start listening")]
    Listen
}

impl Network {
    pub fn listen(config: &Config) -> Result<Self, NetworkError> {
        let (handler, listener) = node::split::<()>();

        let server_addr: SocketAddr = config.listen.parse().unwrap();
        handler.network().listen(Transport::FramedTcp, server_addr).map_err(|_| NetworkError::Listen)?;

        log::info!("Listening on {}", server_addr);

        let (task, receiver) = listener.enqueue();

        Ok(Self {
            handler,
            task,
            receiver,
            peer_map: BiMap::new(),
            endpoints: HashSet::new(),
        })
    }

    pub fn stop(&self) {
        self.handler.stop();
    }

    pub fn send_to(&mut self, endpoint: Endpoint, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        self.handler.network().send(endpoint, &bytes);
    }

    pub fn send(&mut self, entity: Entity, message: &Packet) {
        if let Some(&endpoint) = self.peer_map.get_by_left(&entity) {
            let bytes = rmp_serde::to_vec(&message).unwrap();
            self.handler.network().send(endpoint, &bytes);
        }
    }

    pub fn send_list(&mut self, entity_list: &[Entity], message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for entity in entity_list {
            if let Some(&endpoint) = self.peer_map.get_by_left(entity) {
                self.handler.network().send(endpoint, &bytes);
            }
        }
    }

    pub fn broadcast(&mut self, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for &endpoint in self.peer_map.right_values() {
            self.handler.network().send(endpoint, &bytes);
        }
    }

    pub fn broadcast_except(&mut self, exclude: Entity, message: &Packet) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        for (&entity, &endpoint) in &self.peer_map {
            if entity == exclude {
                continue;
            }
            
            self.handler.network().send(endpoint, &bytes);
        }
    }
}
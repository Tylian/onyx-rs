use std::net::SocketAddr;

use message_io::events::EventReceiver;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, NodeHandler, NodeTask, StoredNodeEvent};
use onyx::network::client::Packet;
use thiserror::Error;

pub struct Network {
    pub handler: NodeHandler<()>,
    pub receiver: EventReceiver<StoredNodeEvent<()>>,
    pub endpoint: Endpoint,

    #[allow(dead_code)] // RAII
    task: NodeTask,
}

#[derive(Clone, Copy, Debug, Error)]
pub enum NetworkError {
    #[error("could not connect")]
    Connect,
}

impl Network {
    pub fn connect(server_addr: SocketAddr) -> Result<Self, NetworkError> {
        let (handler, listener) = node::split::<()>();

        let (server, server_addr) = handler
            .network()
            .connect(Transport::FramedTcp, server_addr)
            .map_err(|_| NetworkError::Connect)?;
        log::info!("Connected to {}", server_addr);

        let (task, receiver) = listener.enqueue();

        Ok(Self {
            handler,
            task,
            receiver,
            endpoint: server,
        })
    }

    pub fn stop(&self) {
        self.handler.stop();
    }

    pub fn send(&mut self, message: &Packet) {
        let bytes = rmp_serde::to_vec(message).unwrap();
        self.handler.network().send(self.endpoint, &bytes);
    }
}

use std::{net::{SocketAddr, UdpSocket}, time::SystemTime};

use common::network::client::Packet;
use renet::{
    transport::{ClientAuthentication, NetcodeClientTransport}, ConnectionConfig, DefaultChannel, RenetClient
};
// use message_io::{
//     events::EventReceiver,
//     network::{Endpoint, ToRemoteAddr, Transport},
//     node::{self, NodeHandler, NodeTask, StoredNodeEvent},
// };


/// Represents an active network connection
pub struct Network {
    pub client: RenetClient,
    pub transport: NetcodeClientTransport,
}

impl Network {
    pub fn connect(server_addr: SocketAddr) -> Self {
        let client = RenetClient::new(ConnectionConfig::default());
        
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id: 0,
            user_data: None,
            protocol_id: 0,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        Self {
            client,
            transport
        }
    }

    pub fn send(&mut self, message: &Packet) {
        let bytes = rmp_serde::to_vec(message).unwrap();
        self.client.send_message(DefaultChannel::ReliableUnordered, bytes);
    }
}

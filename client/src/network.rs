use common::network::ClientMessage;
use message_io::{
    events::EventReceiver,
    network::{Endpoint, ToRemoteAddr, Transport},
    node::{self, NodeHandler, NodeTask, StoredNodeEvent},
};

pub struct Network {
    handler: NodeHandler<()>,
    endpoint: Endpoint,
    receive: EventReceiver<StoredNodeEvent<()>>,

    _task: NodeTask,
}

impl Network {
    pub fn connect(addr: impl ToRemoteAddr) -> Self {
        let (handler, listener) = node::split::<()>();
        let (endpoint, _local_addr) = handler.network().connect(Transport::FramedTcp, addr).unwrap();

        let (_task, receive) = listener.enqueue();

        Self {
            handler,
            endpoint,
            _task,
            receive,
        }
    }

    pub fn try_receive(&mut self) -> Option<StoredNodeEvent<()>> {
        self.receive.try_receive()
    }

    pub fn send(&self, message: ClientMessage) {
        let bytes = rmp_serde::to_vec(&message).unwrap();
        self.handler.network().send(self.endpoint, &bytes);
    }
}

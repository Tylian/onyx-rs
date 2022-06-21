use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::thread;

use message_io::network::{ToRemoteAddr, Transport};
use message_io::node::{self, StoredNetEvent};
use onyx_common::network::{ClientMessage, ServerMessage};

#[derive(Copy, Clone, PartialEq)]
pub enum NetworkStatus {
    NotConnected,
    Connecting,
    Connected,
    Disconnected,
}

pub struct NetworkClient {
    state: Arc<State>,
}

pub struct State {
    status: RwLock<NetworkStatus>,
    buffer: RwLock<VecDeque<ServerMessage>>,
    queue: RwLock<Vec<ClientMessage>>,
    disconnect: AtomicBool,
}

impl State {
    fn new() -> Self {
        Self {
            status: RwLock::new(NetworkStatus::NotConnected),
            buffer: RwLock::new(VecDeque::new()),
            queue: RwLock::new(Vec::new()),
            disconnect: AtomicBool::new(false),
        }
    }

    fn status(&self) -> NetworkStatus {
        *self.status.read().unwrap()
    }

    fn set_status(&self, status: NetworkStatus) {
        *self.status.write().unwrap() = status;
    }

    fn push_buffer(&self, message: ServerMessage) {
        self.buffer.write().unwrap().push_back(message);
    }

    fn try_recv(&self) -> Option<ServerMessage> {
        self.buffer.write().unwrap().pop_front()
    }

    fn queue(&self, message: ClientMessage) {
        self.queue.write().unwrap().push(message);
    }

    fn drain(&self) -> Vec<ClientMessage> {
        std::mem::take(&mut *self.queue.write().unwrap())
    }
}

impl NetworkClient {
    pub fn new() -> Self {
        Self { state: Arc::new(State::new()) }
    }

    pub fn connect(&mut self, addr: impl ToRemoteAddr) {
        let addr = addr.to_remote_addr().unwrap();
        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            state.set_status(NetworkStatus::Connecting);

            let (handler, listener) = node::split::<()>();
            let (server_id, _local_addr) =
                handler.network().connect(Transport::FramedTcp, addr).unwrap();

            let (_task, mut receive) = listener.enqueue();

            loop {
                while let Some(event) = receive.try_receive() {
                    match event.network() {
                        StoredNetEvent::Connected(_, _) => {
                            state.set_status(NetworkStatus::Connected);
                        }
                        StoredNetEvent::Accepted(_, _) => unreachable!(),
                        StoredNetEvent::Message(_, bytes) => {
                            let message =
                                bincode::deserialize(&bytes).unwrap();
                            state.push_buffer(message);
                        }
                        StoredNetEvent::Disconnected(_) => {
                            state.set_status(NetworkStatus::Disconnected);
                        }
                    }
                }

                if state.status() == NetworkStatus::Connected {
                    for data in state.drain() {
                        let bytes = bincode::serialize(&data).unwrap();
                        handler.network().send(server_id, &bytes);
                    }
                }

                if state.disconnect.load(Ordering::Relaxed) {
                    break;
                }
            }

            // clean up
            handler.stop();
        });
    }

    #[inline]
    pub fn status(&self) -> NetworkStatus {
        self.state.status()
    }

    pub fn send(&self, message: ClientMessage) {
        self.state.queue(message);
    }

    pub fn try_recv(&self) -> Option<ServerMessage> {
        self.state.try_recv()
    }

    #[allow(dead_code)]
    pub fn disconnect(&self) {
        self.state.disconnect.store(true, Ordering::Relaxed);
    }
}

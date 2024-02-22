use mint::{Point2, Vector2};
use serde::{Deserialize, Serialize};

use super::{ChatChannel, Map, MapId};

/// Packets sent from the client to the server
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Packet {
    CreateAccount {
        username: String,
        password: String,
        character_name: String,
    },
    Login {
        username: String,
        password: String,
    },
    Move {
        position: Point2<f32>,
        velocity: Vector2<f32>,
    },
    ChatMessage(ChatChannel, String),
    RequestMap,
    SaveMap(Box<Map>),
    Warp(MapId, Option<Point2<f32>>),
    MapEditor(bool),
}

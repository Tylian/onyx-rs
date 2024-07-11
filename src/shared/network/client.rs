use serde::{Deserialize, Serialize};

use super::{ChatChannel, Input, Map, MapId};
use crate::math::units::world::*;

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
    Input(Input),
    ChatMessage(ChatChannel, String),
    RequestMap,
    SaveMap(Box<Map>),
    Warp(MapId, Option<Point2D>),
    MapEditor(bool),
}

use std::{collections::HashMap, fmt::Display};

use mint::{Point2, Vector2};
use serde::{Deserialize, Serialize};

use super::{ChatChannel, ClientId, Direction, Map, MapSettings, Player, PlayerFlags, MapId};

/// Packets sent from the server to the client
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Packet {
    JoinGame(ClientId),
    FailedJoin(FailJoinReason),
    PlayerData(ClientId, Player),
    RemoveData(ClientId),
    PlayerMove {
        client_id: ClientId,
        position: Point2<f32>,
        direction: Direction,
        velocity: Option<Vector2<f32>>,
    },
    ChatLog(ChatChannel, String),
    ChangeMap(MapId, i64),
    MapData(Box<Map>),
    MapEditor {
        maps: HashMap<MapId, String>,
        id: MapId,
        width: u32,
        height: u32,
        settings: MapSettings,
    },
    Flags(ClientId, PlayerFlags),
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum FailJoinReason {
    UsernameTaken,
    CharacterNameTaken,
    LoginIncorrect,
}

impl Display for FailJoinReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailJoinReason::UsernameTaken => write!(f, "username is taken"),
            FailJoinReason::CharacterNameTaken => write!(f, "character name is taken"),
            FailJoinReason::LoginIncorrect => write!(f, "username/password is incorrect"),
        }
    }
}

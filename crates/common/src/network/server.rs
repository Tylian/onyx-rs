use std::{collections::HashMap, fmt::Display};

use mint::{Point2, Vector2};
use serde::{Deserialize, Serialize};

use super::{ChatChannel, Entity, Direction, Map, MapSettings, Player, PlayerFlags, MapId};

/// Packets sent from the server to the client
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Packet {
    JoinGame(Entity),
    FailedJoin(FailJoinReason),
    PlayerData(Entity, Player),
    RemoveData(Entity),
    PlayerMove {
        entity: Entity,
        position: Point2<f32>,
        velocity: Vector2<f32>,
    },
    ChatLog(ChatChannel, String),
    ChangeMap(MapId, u64),
    MapData(Box<Map>),
    MapEditor {
        maps: HashMap<MapId, String>,
        id: MapId,
        width: u32,
        height: u32,
        settings: MapSettings,
    },
    Flags(Entity, PlayerFlags),
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

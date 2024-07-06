use std::{collections::HashMap, fmt::Display};
use serde::{Deserialize, Serialize};

use crate::math::units::world::*;
use crate::network::{ChatChannel, Entity, Map, MapSettings, Player, PlayerFlags, MapId};

/// Packets sent from the server to the client
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Packet {
    JoinGame(Entity),
    FailedJoin(FailJoinReason),
    PlayerData(Entity, Player),
    RemoveData(Entity),
    PlayerMove {
        entity: Entity,
        position: Point2D,
        velocity: Vector2D,
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

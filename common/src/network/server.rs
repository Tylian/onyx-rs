use std::{fmt::Display, collections::HashMap};

use mint::{Point2, Vector2};
use serde::{Serialize, Deserialize};

use super::{Player, ClientId, ChatChannel, Direction, Map, MapHash, MapSettings, PlayerFlags};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Packet {
    JoinGame(ClientId),
    FailedJoin(FailJoinReason),
    PlayerJoined(ClientId, Player),
    PlayerLeft(ClientId),
    PlayerMove {
        client_id: ClientId,
        position: Point2<f32>,
        direction: Direction,
        velocity: Option<Vector2<f32>>,
    },
    ChatLog(ChatChannel, String),
    ChangeMap(MapHash, i64),
    MapData(Box<Map>),
    MapEditor {
        maps: HashMap<String, String>,
        id: String,
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
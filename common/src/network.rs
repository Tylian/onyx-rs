use std::{collections::HashMap, fmt::Display};

use mint::{Point2, Vector2};
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, IntoEnumIterator};

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
pub struct ClientId(pub u64);

impl From<u64> for ClientId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ClientMessage {
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
        direction: Direction,
        velocity: Option<Vector2<f32>>,
    },
    Message(String),
    RequestMap,
    SaveMap(Box<Map>),
    Warp(MapId, Option<Point2<f32>>),
    MapEditor,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ServerMessage {
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
    Message(ChatMessage),
    ChangeMap(MapId, u32),
    MapData(Box<Map>),
    MapEditor {
        maps: HashMap<MapId, String>,
        id: MapId,
        width: u32,
        height: u32,
        settings: MapSettings,
    },
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

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Direction {
    South,
    West,
    East,
    North,
}

impl Direction {
    pub fn reverse(&self) -> Self {
        match self {
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
            Self::North => Self::South,
        }
    }
    pub fn offset_f32(&self) -> Vector2<f32> {
        match self {
            Direction::South => Vector2 { x: 0., y: 1. },
            Direction::West => Vector2 { x: -1., y: 0. },
            Direction::East => Vector2 { x: 1., y: 0. },
            Direction::North => Vector2 { x: 0., y: -1. },
        }
    }
    pub fn offset_i32(&self) -> Vector2<i32> {
        match self {
            Direction::South => Vector2 { x: 0, y: 1 },
            Direction::West => Vector2 { x: -1, y: 0 },
            Direction::East => Vector2 { x: 1, y: 0 },
            Direction::North => Vector2 { x: 0, y: -1 },
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::South => write!(f, "South"),
            Direction::West => write!(f, "West"),
            Direction::East => write!(f, "East"),
            Direction::North => write!(f, "North"),
        }
    }
}

impl From<Direction> for Vector2<f32> {
    fn from(dir: Direction) -> Self {
        dir.offset_f32()
    }
}

impl From<Direction> for Vector2<i32> {
    fn from(dir: Direction) -> Self {
        dir.offset_i32()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Player {
    pub name: String,
    pub position: Point2<f32>,
    pub sprite: u32,
    pub direction: Direction,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ChatMessage {
    Server(String),
    Say(String),
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug, Eq, Hash, EnumCount, EnumIter)]
pub enum MapLayer {
    Ground,
    Mask,
    Mask2,
    Fringe,
    Fringe2,
}

impl Display for MapLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MapLayer::Ground => "Ground",
                MapLayer::Mask => "Mask",
                MapLayer::Mask2 => "Mask 2",
                MapLayer::Fringe => "Fringe",
                MapLayer::Fringe2 => "Fringe 2",
            }
        )
    }
}

#[derive(Default, Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
pub struct MapId(pub u64);

impl From<u64> for MapId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl MapId {
    /// Returns the special, must always exist map
    pub fn start() -> MapId {
        MapId(0)
    }
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Map {
    pub id: MapId,
    pub width: u32,
    pub height: u32,
    pub settings: MapSettings,
    pub layers: HashMap<MapLayer, Array2<Option<Tile>>>,
    pub zones: Vec<Zone>,
}

impl Map {
    pub fn new(id: MapId, width: u32, height: u32) -> Self {
        let settings = MapSettings::default();
        let mut layers = HashMap::new();
        let zones = Vec::new();

        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((width as usize, height as usize)));
        }

        Self {
            id,
            width,
            height,
            settings,
            layers,
            zones,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct MapSettings {
    pub name: String,
    pub tileset: String,
    pub music: Option<String>,
    pub warps: BoundryWarps,
    pub revision: u32,
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            tileset: String::from("default.png"),
            music: None,
            warps: BoundryWarps::default(),
            revision: 0,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct BoundryWarps {
    pub north: Option<MapId>,
    pub east: Option<MapId>,
    pub south: Option<MapId>,
    pub west: Option<MapId>,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Tile {
    pub texture: Point2<i32>,
    pub autotile: bool,
    pub animation: Option<TileAnimation>,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct TileAnimation {
    pub frames: u16,
    pub duration: f64,
    pub bouncy: bool,
}

impl TileAnimation {
    pub fn total_frames(&self) -> u16 {
        if self.bouncy {
            self.frames * 2 - 1
        } else {
            self.frames
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ZoneData {
    Blocked,
    Warp(MapId, Point2<f32>, Option<Direction>),
}

impl ZoneData {
    pub fn name(&self) -> &str {
        match self {
            ZoneData::Blocked => "Blocked",
            ZoneData::Warp(_, _, _) => "Warp",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Zone {
    pub position: Point2<f32>,
    pub size: Vector2<f32>,
    pub data: ZoneData,
}

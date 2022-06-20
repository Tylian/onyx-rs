use std::{collections::HashMap, fmt::Display};

use serde::{Serialize, Deserialize};
use enum_iterator::Sequence;
use ndarray::Array2;

use mint::{Point2, Vector2};

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
    Move { position: Point2<f32>, direction: Direction, velocity: Option<Vector2<f32>> },
    Hello(String, u32),
    Message(String),
    RequestMap,
    SaveMap(Map)
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ServerMessage {
    Hello(ClientId),
    PlayerJoined(ClientId, PlayerData),
    PlayerLeft(ClientId),
    PlayerMoved { client_id: ClientId, position: Point2<f32>, direction: Direction, velocity: Option<Vector2<f32>> },
    Message(ChatMessage),
    ChangeMap(Map),
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
pub struct PlayerData {
    pub name: String,
    pub position: Point2<f32>,
    pub sprite: u32,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ChatMessage {
    Server(String),
    Say(String)
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Sequence)]
pub enum MapLayer {
    Ground,
    Mask,
    Mask2,
    Fringe,
    Fringe2
}

impl MapLayer {
    pub fn iter() -> impl Iterator<Item = MapLayer> {
        enum_iterator::all::<Self>()
    }

    pub fn count() -> usize {
        enum_iterator::cardinality::<Self>()
    }
}

impl Display for MapLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            MapLayer::Ground => "Ground",
            MapLayer::Mask => "Mask",
            MapLayer::Mask2 => "Mask 2",
            MapLayer::Fringe => "Fringe",
            MapLayer::Fringe2 => "Fringe 2",
        })
    }
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Map {
    pub width: u32,
    pub height: u32,
    pub layers: HashMap<MapLayer, Array2<Option<Tile>>>,
    pub areas: Vec<Area>
}

impl Map {
    pub fn new(width: u32, height: u32) -> Self {
        let mut layers = HashMap::new();

        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((width as usize, height as usize)));
        }

        Self {
            width,
            height,
            layers,
            areas: Vec::new(),
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Tile {
    pub texture: Point2<i32>,
    pub autotile: bool,
    pub animation_frames: u8,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum AreaData {
    Blocked,
    Log(String),
}

impl AreaData {
    pub fn name(&self) -> &str {
        match self {
            AreaData::Blocked => "Blocked",
            AreaData::Log(_) => "Log",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Area {
    pub position: Point2<f32>,
    pub size: Vector2<f32>,
    pub data: AreaData,
}
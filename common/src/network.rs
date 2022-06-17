use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use enum_iterator::Sequence;

use crate::{Point2, Vector2, vector2};

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
    Move(Direction),
    Hello(String, u32),
    Message(String),
    ChangeTile { position: Point2, layer: MapLayer, tile: Option<Point2>, is_autotile: bool },
    RequestMap,
    SaveMap(RemoteMap)
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ServerMessage {
    Hello(ClientId),
    PlayerJoined(ClientId, PlayerData),
    PlayerLeft(ClientId),
    PlayerMoved { client_id: ClientId, position: Point2, direction: Direction },
    Message(ChatMessage),
    ChangeTile { position: Point2, layer: MapLayer, tile: Option<Point2>, is_autotile: bool },
    ChangeMap(RemoteMap),
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
    pub fn offset(&self) -> Vector2 {
        match self {
            Direction::South => vector2(0, 1),
            Direction::West => vector2(-1, 0),
            Direction::East => vector2(1, 0),
            Direction::North => vector2(0, -1),
        }
    }
}

impl From<Direction> for Vector2 {
    fn from(dir: Direction) -> Self {
       dir.offset()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct PlayerData {
    pub name: String,
    pub position: Point2,
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
    Fringe,
}

impl MapLayer {
    pub fn iter() -> impl Iterator<Item = MapLayer> {
        enum_iterator::all::<Self>()
    }

    pub fn count() -> usize {
        enum_iterator::cardinality::<Self>()
    }
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct RemoteMap {
    pub width: u32,
    pub height: u32,
    pub layers: HashMap<MapLayer, Vec<RemoteTile>>,
    pub attributes: Vec<TileAttribute>
}

impl RemoteMap {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height).try_into().unwrap();
        let mut layers = HashMap::new();

        for layer in MapLayer::iter() {
            layers.insert(layer, vec![Default::default(); size]);
        }

        Self {
            width,
            height,
            layers,
            attributes: vec![Default::default(); size],
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum RemoteTile {
    Empty,
    Basic(Point2),
    Autotile(Point2),
}

impl Default for RemoteTile {
    fn default() -> Self {
        RemoteTile::Empty
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum TileAttribute {
    None,
    Blocked,
}

impl Default for TileAttribute {
    fn default() -> Self {
        TileAttribute::None
    }
}
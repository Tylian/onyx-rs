use std::collections::HashMap;
use std::fmt::Display;

use euclid::approxeq::ApproxEq;
use glam::{ivec2, vec2, IVec2, UVec2, Vec2};
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, IntoEnumIterator};

use crate::math::units::map;
use crate::math::units::world::{self, *};
use crate::{LERP_DURATION, RUN_SPEED, WALK_SPEED};

pub mod client;
pub mod server;

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
pub struct Entity(pub u64);

impl Entity {
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }

    pub fn raw(&self) -> u64 {
        self.0
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, Copy, Default, PartialOrd, Ord)]
#[serde(transparent)]
pub struct MapId(pub u64);

impl MapId {
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }

    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl From<u64> for MapId {
    fn from(id: u64) -> Self {
        Self(id)
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
    pub fn from_velocity(velocity: Vector2D) -> Option<Self> {
        if velocity.approx_eq(&Vector2D::zero()) {
            return None;
        }

        if velocity.x.abs().approx_eq(&velocity.y.abs()) {
            return None;
        }

        #[allow(clippy::collapsible_else_if)] // visual logic more important
        if velocity.x.abs() > velocity.y.abs() {
            if velocity.x > 0.0 {
                Some(Direction::East)
            } else {
                Some(Direction::West)
            }
        } else {
            if velocity.y > 0.0 {
                Some(Direction::South)
            } else {
                Some(Direction::North)
            }
        }
    }

    #[must_use]
    pub fn reverse(&self) -> Self {
        match self {
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
            Self::North => Self::South,
        }
    }
    pub fn offset_f32(&self) -> Vec2 {
        match self {
            Direction::South => vec2(0.0, 1.0),
            Direction::West => vec2(-1.0, 0.0),
            Direction::East => vec2(1.0, 0.0),
            Direction::North => vec2(0.0, -1.0),
        }
    }
    pub fn offset_i32(&self) -> IVec2 {
        match self {
            Direction::South => ivec2(0, 1),
            Direction::West => ivec2(-1, 0),
            Direction::East => ivec2(1, 0),
            Direction::North => ivec2(0, -1),
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

impl From<Direction> for Vec2 {
    fn from(dir: Direction) -> Self {
        dir.offset_f32()
    }
}

impl From<Direction> for IVec2 {
    fn from(dir: Direction) -> Self {
        dir.offset_i32()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Player {
    pub name: String,
    pub position: Point2D,
    pub velocity: Vector2D,
    pub map: MapId,
    pub sprite: u32,
    pub direction: Direction,
    pub flags: PlayerFlags,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct PlayerFlags {
    pub in_map_editor: bool,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ChatChannel {
    Echo,
    Server,
    Say,
    Global,
    Error,
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

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Map {
    pub id: MapId,
    pub size: map::Size2D,
    pub settings: MapSettings,
    pub layers: HashMap<MapLayer, Array2<Option<Tile>>>,
    pub zones: Vec<Zone>,
}

impl Map {
    pub fn new(id: MapId, size: map::Size2D) -> Self {
        let settings = MapSettings::default();
        let mut layers = HashMap::new();
        let zones = Vec::new();

        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((size.width as usize, size.height as usize)));
        }

        Self {
            id,
            size,
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
    pub cache_key: u64,
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            tileset: String::from("default.png"),
            music: None,
            warps: BoundryWarps::default(),
            cache_key: 0,
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

impl BoundryWarps {
    pub fn iter(&self) -> impl Iterator<Item = (Direction, Option<MapId>)> {
        let vec = vec![
            (Direction::North, self.north),
            (Direction::East, self.east),
            (Direction::South, self.south),
            (Direction::West, self.west),
        ];
        vec.into_iter()
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Tile {
    pub texture: UVec2,
    pub autotile: bool,
    pub animation: Option<TileAnimation>,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct TileAnimation {
    pub frames: u32,
    pub duration: f32,
    pub bouncy: bool,
}

impl TileAnimation {
    pub fn total_frames(&self) -> u32 {
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
    Warp(MapId, Point2D, Option<Direction>),
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
    pub position: world::Box2D,
    pub data: ZoneData,
}

#[derive(Clone, Copy, Debug)]
pub struct Interpolation {
    pub source: State,
    pub target: State,
    pub start: f32,
}

impl Interpolation {
    pub fn lerp(&self, time: f32) -> State {
        let progress = f32::clamp((time - self.start) / LERP_DURATION, 0.0, 1.0);
        self.source.lerp(&self.target, progress)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub id: Entity,
    pub position: Point2D,
    pub velocity: Vector2D,
    pub direction: Direction,
    pub map: MapId,
    pub max_speed: f32,
    pub last_sequence_id: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Input {
    pub acceleration: Vector2D,
    pub running: bool,
    pub sequence_id: u64,
    pub dt: f32,
}

impl State {
    pub fn from_input(&self, input: Input, test_friction: f32) -> State {
        let mut next = *self;
        next.apply_input(input, test_friction);
        next
    }

    pub fn apply_input(&mut self, input: Input, test_friction: f32) {
        self.max_speed = if input.running { RUN_SPEED } else { WALK_SPEED };

        let velocity = (self.velocity + input.acceleration).clamp_length(0.0, self.max_speed);
        let friction_force = velocity.try_normalize().unwrap_or_default() * test_friction * input.dt;

        self.velocity = if friction_force.square_length() <= velocity.square_length() {
            velocity - friction_force
        } else {
            Vector2D::zero()
        };

        if self.velocity.square_length() >= f32::EPSILON * f32::EPSILON {
            self.position += self.velocity * input.dt;
        }

        self.last_sequence_id = input.sequence_id;
    }

    pub fn lerp(&self, other: &State, t: f32) -> State {
        let position = self.position.lerp(other.position, t);
        let velocity = (other.position - self.position) / LERP_DURATION;

        State {
            id: other.id,
            position,
            velocity,
            direction: Direction::from_velocity(velocity).unwrap_or(Direction::South),
            map: other.map,
            max_speed: other.max_speed,
            last_sequence_id: other.last_sequence_id,
        }
    }
}

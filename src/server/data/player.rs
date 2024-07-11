use std::path::PathBuf;

use anyhow::Result;
use onyx::network::{Direction, Entity, Input, MapId, Player as NetworkPlayer, PlayerFlags, State};
use onyx::math::units::world::*;
use onyx::{RUN_SPEED, SPRITE_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub username: String,
    pub password: String,
    pub name: String,
    pub sprite: u32,
    pub map: MapId,
    pub position: Point2D,
    pub direction: Direction,
}

impl PlayerData {
    pub fn path(name: &str) -> PathBuf {
        PathBuf::from(format!("players/{name}.toml"))
    }

    pub fn load(name: &str) -> Result<Self> {
        let path = Self::path(name);

        let contents = std::fs::read_to_string(path)?;
        let player = toml::from_str(&contents)?;

        Ok(player)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path(&self.username);

        let contents = toml::to_string_pretty(&self)?;
        std::fs::write(path, contents)?;

        Ok(())
    }
}

impl From<Player> for PlayerData {
    fn from(other: Player) -> Self {
        Self {
            username: other.username,
            password: other.password,
            name: other.name,
            sprite: other.sprite,
            map: other.map,
            position: other.position,
            direction: other.direction,
        }
    }
}

#[derive(Clone)]
pub struct Player {
    pub id: Entity,

    pub username: String,
    pub password: String,
    pub name: String,
    pub sprite: u32,
    pub map: MapId,
    pub position: Point2D,
    pub direction: Direction,

    pub velocity: Vector2D,
    pub last_movement_update: f32,
    pub flags: PlayerFlags,
    pub inputs: Vec<Input>,
    pub last_sequence_id: u64,
    pub max_speed: f32,
}

impl From<Player> for NetworkPlayer {
    fn from(other: Player) -> Self {
        Self {
            name: other.name,
            sprite: other.sprite,
            velocity: other.velocity,
            position: other.position,
            map: other.map,
            direction: other.direction,
            flags: other.flags,
        }
    }
}

impl Player {
    pub fn path(name: &str) -> PathBuf {
        PathBuf::from(format!("players/{name}.toml"))
    }

    pub fn from_data(id: Entity, data: PlayerData) -> Self {
        Self {
            id,
            username: data.username,
            password: data.password,
            name: data.name,
            sprite: data.sprite,
            map: data.map,
            position: data.position,
            direction: data.direction,
            velocity: Vector2D::zero(),
            last_movement_update: 0.0,
            flags: PlayerFlags::default(),
            inputs: Vec::new(),
            last_sequence_id: 0,
            max_speed: RUN_SPEED,
        }
    }
    
    pub fn state(&self) -> State {
        State {
            id: self.id,
            position: self.position,
            velocity: self.velocity,
            max_speed: self.max_speed,
            last_sequence_id: self.last_sequence_id,
            direction: self.direction,
            map: self.map,
        }
    }

    pub fn apply_state(&mut self, state: State) {
        self.position = state.position;
        self.velocity = state.velocity;
        self.max_speed = state.max_speed;
        self.last_sequence_id = state.last_sequence_id;
        self.direction = state.direction;
        self.map = state.map;
    }
}

impl Player {
    pub fn new(id: Entity, username: &str, password: &str, name: &str, map: MapId, position: Point2D) -> Self {
        Self {
            id,
            username: username.into(),
            password: password.into(),
            name: name.into(),
            sprite: 0,
            position,
            direction: Direction::South,
            map,
            velocity: Vector2D::zero(),
            flags: PlayerFlags::default(),
            last_movement_update: 0.0,
            inputs: Vec::new(),
            last_sequence_id: 0,
            max_speed: RUN_SPEED,
        }
    }

    // only block on the bottom half of the sprite, feels better
    pub fn collision_box(position: Point2D) -> Box2D {
        Box2D::from_origin_and_size(
            position + Vector2D::new(0.0, SPRITE_SIZE / 2.0),
            Size2D::new(SPRITE_SIZE, SPRITE_SIZE / 2.0)
        )
    }
}
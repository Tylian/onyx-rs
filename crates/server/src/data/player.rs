use std::path::PathBuf;

use anyhow::Result;
use common::network::{Direction, Player as NetworkPlayer, PlayerFlags, MapId};
use euclid::default::{Point2D, Vector2D};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub username: String,
    pub password: String,
    pub name: String,
    pub sprite: u32,
    pub map: MapId,
    pub position: Point2D<f32>,
    pub direction: Direction,
    #[serde(skip)]
    pub velocity: Vector2D<f32>,

    #[serde(skip)]
    pub last_movement_update: f32, // 

    #[serde(skip)]
    pub flags: PlayerFlags,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            name: String::new(),
            sprite: 0,
            position: Point2D::new(0.0, 0.0),
            direction: Direction::South,
            map: MapId::default(),
            flags: PlayerFlags::default(),
            velocity: Vector2D::zero(),
            last_movement_update: 0.0,
        }
    }
}

impl From<Player> for NetworkPlayer {
    fn from(other: Player) -> Self {
        Self {
            name: other.name,
            sprite: other.sprite,
            velocity: other.velocity.into(),
            position: other.position.into(),
            direction: other.direction,
            flags: other.flags,
        }
    }
}

impl Player {
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

        let player = self.clone();
        let contents = toml::to_string_pretty(&player)?;
        std::fs::write(path, contents)?;

        Ok(())
    }
}

impl Player {
    pub fn new(username: &str, password: &str, name: &str, map: MapId, position: Point2D<f32>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            name: name.into(),
            sprite: 0,
            position,
            direction: Direction::South,
            map,
            velocity: Vector2D::zero(),
            flags: PlayerFlags::default(),
            last_movement_update: 0.0
        }
    }
}

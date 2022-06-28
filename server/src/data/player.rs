use std::path::PathBuf;

use crate::player::Player as PlayerState;
use anyhow::Result;
use common::network::{Direction, MapId, PlayerFlags};
use euclid::default::Point2D;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub username: String,
    pub password: String,
    pub name: String,
    pub sprite: u32,
    pub position: Point2D<f32>,
    pub direction: Direction,
    pub map: MapId,
    pub flags: PlayerFlags
}

impl Default for Player {
    fn default() -> Self {
        Self {
            username: Default::default(),
            password: Default::default(),
            name: Default::default(),
            sprite: Default::default(),
            position: Default::default(),
            direction: Direction::South,
            map: MapId::start(),
            flags: PlayerFlags::default(),
        }
    }
}

impl From<Player> for PlayerState {
    fn from(other: Player) -> Self {
        Self {
            username: other.username,
            password: other.password,
            name: other.name,
            sprite: other.sprite,
            position: other.position,
            direction: other.direction,
            velocity: None,
            map: other.map,
            flags: other.flags
        }
    }
}

impl From<PlayerState> for Player {
    fn from(other: PlayerState) -> Self {
        Self {
            username: other.username,
            password: other.password,
            name: other.name,
            sprite: other.sprite,
            position: other.position,
            direction: other.direction,
            map: other.map,
            flags: other.flags,
        }
    }
}

impl Player {
    pub fn path(name: &str) -> PathBuf {
        let mut path = common::server_runtime!();
        path.push("players");
        path.push(format!("{name}.toml"));
        path
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

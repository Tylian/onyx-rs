use std::path::PathBuf;

use anyhow::{Result, Context};
use common::network::{Direction, MapId, PlayerFlags, Player as NetworkPlayer};
use euclid::default::{Point2D, Vector2D};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub username: String,
    pub password: String,
    pub name: String,
    pub sprite: u32,
    pub position: Point2D<f32>,
    pub direction: Direction,
    #[serde(skip)]
    pub velocity: Option<Vector2D<f32>>,
    pub map: MapId,

    #[serde(skip)]
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
            velocity: Default::default(),
        }
    }
}

impl From<Player> for NetworkPlayer {
    fn from(other: Player) -> Self {
        Self {
            name: other.name,
            sprite: other.sprite,
            velocity: other.velocity.map(Into::into),
            position: other.position.into(),
            direction: other.direction,
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

        let contents = std::fs::read_to_string(path).context("Read")?;
        let player = toml::from_str(&contents).context("Parse")?;

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
            velocity: None,
            flags: PlayerFlags::default(),
        }
    }
}

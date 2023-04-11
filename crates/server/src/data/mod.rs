mod map;
mod player;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Result;
use euclid::default::Point2D;
use serde::{Deserialize, Serialize};

pub use self::map::*;
pub use self::player::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub listen: String,
    pub start: Start,
}

impl Config {
    pub fn path() -> PathBuf {
        common::server_path("config.toml")
    }
    pub fn load() -> Result<Self> {
        let contents = std::fs::read_to_string(Self::path())?;
        Ok(toml::from_str(&contents)?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Start {
    pub x: f32,
    pub y: f32,
}

impl Start {
    pub fn position(&self) -> Point2D<f32> {
        Point2D::new(self.x, self.y)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NameCache {
    names: HashSet<String>,
}

impl NameCache {
    pub fn path() -> PathBuf {
        common::server_path("names.cache")
    }
    pub fn load() -> Result<Self> {
        use std::io::{ErrorKind, Result};
        let contents = match std::fs::read_to_string(Self::path()) {
            Ok(contents) => Result::Ok(contents),
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(NameCache::default()),
            Err(e) => Result::Err(e),
        }?;

        Ok(Self {
            names: contents.lines().map(ToString::to_string).collect(),
        })
    }
    pub fn save(&self) -> Result<()> {
        let contents = self.names.iter().cloned().collect::<Vec<_>>().join("\n");

        std::fs::write(Self::path(), contents)?;
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }
    pub fn insert(&mut self, name: String) {
        self.names.insert(name);
    }
}

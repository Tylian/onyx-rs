mod map;
mod player;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Result;
use common::network::MapId;
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
        let mut path = common::server_runtime!();
        path.push("config.toml");

        path
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
    pub map: MapId,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NameCache {
    names: HashSet<String>,
}

impl NameCache {
    pub fn path() -> PathBuf {
        let mut path = common::server_runtime!();
        path.push("names.cache");

        path
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

        std::fs::write(Self::path(), &contents)?;
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }
    pub fn insert(&mut self, name: String) {
        self.names.insert(name);
    }
}

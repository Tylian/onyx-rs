use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use common::{
    network::{Map as NetworkMap, MapId, MapLayer, MapSettings, Tile, Zone},
    TILE_SIZE,
};
use euclid::default::Box2D;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

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
    pub fn path(id: MapId) -> PathBuf {
        let mut path = common::server_runtime!();
        path.push("maps");
        path.push(format!("{}.bin", id.0));
        path
    }

    pub fn load(id: MapId) -> Result<Self> {
        let path = Self::path(id);
        Self::load_path(path)
    }

    pub fn load_all() -> Result<HashMap<MapId, Self>> {
        let mut path = common::server_runtime!();
        path.push("maps");

        let mut maps = HashMap::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let map = Self::load_path(&path)?;

                #[cfg(debug_assertions)]
                if path.file_name().and_then(std::ffi::OsStr::to_str) != Some(&format!("{}.bin", map.id.0)) {
                    log::warn!(
                        "Map loaded but the name didn't match it's id: {:?} {}",
                        map.id,
                        path.display()
                    );
                }

                maps.insert(map.id, map);
            }
        }

        Ok(maps)
    }

    fn load_path(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read(path)?;
        let map = bincode::deserialize(&contents)?;

        Ok(map)
    }

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

    pub fn save(&self) -> Result<()> {
        let path = Self::path(self.id);

        let map = self.clone();
        let contents = bincode::serialize(&map)?;
        std::fs::write(path, contents)?;

        Ok(())
    }

    pub fn to_box2d(&self) -> Box2D<f32> {
        use euclid::default::{Point2D, Rect, Size2D};

        Rect::new(
            Point2D::zero(),
            Size2D::new(
                self.width as f32 * TILE_SIZE as f32,
                self.height as f32 * TILE_SIZE as f32,
            ),
        )
        .to_box2d()
    }
}

impl From<NetworkMap> for Map {
    fn from(other: NetworkMap) -> Self {
        Self {
            id: other.id,
            width: other.width,
            height: other.height,
            settings: other.settings,
            layers: other.layers,
            zones: other.zones,
        }
    }
}

impl From<Map> for NetworkMap {
    fn from(other: Map) -> Self {
        Self {
            id: other.id,
            width: other.width,
            height: other.height,
            settings: other.settings,
            layers: other.layers,
            zones: other.zones,
        }
    }
}

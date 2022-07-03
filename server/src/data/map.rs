use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use common::{
    network::{Map as NetworkMap, MapHash, MapLayer, MapSettings, Tile, Zone},
    TILE_SIZE,
};
use euclid::default::Box2D;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Map {
    #[serde(alias = "id")]
    pub hash: MapHash,
    #[serde(skip)]
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub settings: MapSettings,
    pub layers: HashMap<MapLayer, Array2<Option<Tile>>>,
    pub zones: Vec<Zone>,
}

impl Map {
    pub fn path(id: &str) -> PathBuf {
        let mut path = common::server_runtime!();
        path.push("maps");
        path.push(format!("{}.bin", id));
        path
    }

    pub fn load(id: &str) -> Result<Self> {
        let path = Self::path(id);
        Self::load_path(path)
    }

    pub fn load_all() -> Result<HashMap<MapHash, Self>> {
        let mut path = common::server_runtime!();
        path.push("maps");

        let mut maps = HashMap::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let mut map = Self::load_path(&path)?;
                let id = path.file_stem().unwrap().to_string_lossy();
                let hash = MapHash::from(&*id);

                if map.hash != hash {
                    log::warn!(
                        "Map loaded but the file name didn't match it's hash: {:#x} {:#x}",
                        map.hash.0,
                        hash.0
                    );

                    if cfg!(debug_assertions) {
                        log::debug!("Updating the map's hash to match the file path, this may break warps.");
                        map.hash = hash;
                    }
                }

                maps.insert(map.hash, map);
            }
        }

        Ok(maps)
    }

    fn load_path(path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let map = rmp_serde::from_read(file)?;

        Ok(map)
    }

    pub fn new(id: &str, width: u32, height: u32) -> Self {
        let settings = MapSettings::default();
        let mut layers = HashMap::new();
        let zones = Vec::new();
        let hash = MapHash::from(id);

        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((width as usize, height as usize)));
        }

        Self {
            id: id.to_string(),
            hash,
            width,
            height,
            settings,
            layers,
            zones,
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path(&self.id);

        let map = self.clone();
        let contents = rmp_serde::to_vec_named(&map)?;
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
            hash: other.hash,
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
            hash: other.hash,
            width: other.width,
            height: other.height,
            settings: other.settings,
            layers: other.layers,
            zones: other.zones,
        }
    }
}

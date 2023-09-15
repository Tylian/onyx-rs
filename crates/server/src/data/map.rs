use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Result, Context};
use common::{
    network::{Map as NetworkMap, MapLayer, MapSettings, Tile, Zone, MapId},
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
        common::server_path(format!("maps/{}.bin", id.0))
    }

    pub fn load(id: MapId) -> Result<Self> {
        let path = Self::path(id);
        Self::load_from_file(path)
    }

    pub fn load_all() -> Result<HashMap<MapId, Self>> {
        let path = common::server_path("maps");

        let mut maps = HashMap::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let map = Self::load_from_file(&path).with_context(|| format!("loading {}", path.display()))?;
                maps.insert(map.id, map);
            }
        }

        Ok(maps)
    }

    fn load_from_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path> + Clone,
    {
        let file = std::fs::File::open(path)?;
        let map: Self = rmp_serde::from_read(file)?;

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

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Result, Context};
use onyx::{
    math::units::{map::*, world}, network::{Map as NetworkMap, MapId, MapLayer, MapSettings, Tile, Zone}, TILE_SIZE
};
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Map {
    pub id: MapId,
    pub size: Size2D,
    pub settings: MapSettings,
    pub layers: HashMap<MapLayer, Array2<Option<Tile>>>,
    pub zones: Vec<Zone>,
}

impl Map {
    pub fn path(id: MapId) -> PathBuf {
        PathBuf::from(format!("maps/{}.bin", id.0))
    }

    pub fn load(id: MapId) -> Result<Self> {
        let path = Self::path(id);
        Self::load_from_file(path)
    }

    pub fn load_all() -> Result<HashMap<MapId, Self>> {
        let path = PathBuf::from("maps");

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

    pub fn new(id: MapId, size: Size2D) -> Self {
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

    pub fn save(&self) -> Result<()> {
        let path = Self::path(self.id);

        let map = self.clone();
        let contents = rmp_serde::to_vec_named(&map)?;
        std::fs::write(path, contents)?;

        Ok(())
    }

    pub fn world_size(&self) -> world::Size2D {
        (self.size.to_f32() * TILE_SIZE).cast_unit()
    }

    pub fn valid(&self, pos: Point2D) -> bool {
        Box2D::from_size(self.size).contains(pos)
    }
}

impl From<NetworkMap> for Map {
    fn from(other: NetworkMap) -> Self {
        Self {
            id: other.id,
            size: other.size,
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
            size: other.size,
            settings: other.settings,
            layers: other.layers,
            zones: other.zones,
        }
    }
}

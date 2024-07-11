use std::collections::HashMap;

use onyx::network::{Map as NetworkMap, MapLayer, Tile as NetworkTile};
use ndarray::Array2;
use strum::EnumCount;
use thiserror::Error;

use crate::ensure;

use super::{Map, Tile};

#[derive(Debug, Error)]
pub enum MapError {
    #[error("size is incorrect")]
    IncorrectSize,
    #[error("the number of layers is incorrect")]
    IncorrectLayers,
}

impl TryFrom<NetworkMap> for Map {
    type Error = MapError;

    fn try_from(other: NetworkMap) -> Result<Self, Self::Error> {
        ensure!(other.layers.len() == MapLayer::COUNT, MapError::IncorrectLayers);

        let mut layers = HashMap::new();
        let mut autotiles = HashMap::new();
        for (layer, contents) in other.layers {
            ensure!(
                contents.dim() == (other.size.width as usize, other.size.height as usize),
                MapError::IncorrectSize
            );
            layers.insert(layer, contents.map(|t| t.map(Into::into)));
            autotiles.insert(layer, Array2::default(contents.dim()));
        }

        let mut map = Self {
            id: other.id,
            size: other.size,
            settings: other.settings,
            layers,
            autotiles,
            zones: other.zones.into_iter().map(Into::into).collect(),
        };

        map.update_autotile_cache();
        Ok(map)
    }
}

// Note: It is considered an unrecoverable error to have a map that has an invalid size
impl From<Map> for NetworkMap {
    fn from(other: Map) -> Self {
        let size = (other.size.width * other.size.height) as usize;
        assert_eq!(other.layers.len(), MapLayer::COUNT);

        let mut layers = HashMap::new();
        for (layer, contents) in other.layers {
            assert_eq!(contents.len(), size);
            layers.insert(layer, contents.map(|t| t.map(Into::into)));
        }

        Self {
            id: other.id,
            size: other.size,
            settings: other.settings,
            layers,
            zones: other.zones.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Tile> for NetworkTile {
    fn from(tile: Tile) -> Self {
        Self {
            texture: tile.texture,
            autotile: tile.autotile,
            animation: tile.animation,
        }
    }
}

impl From<NetworkTile> for Tile {
    fn from(tile: NetworkTile) -> Self {
        Self {
            texture: tile.texture,
            autotile: tile.autotile,
            animation: tile.animation,
        }
    }
}

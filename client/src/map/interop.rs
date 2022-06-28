use std::collections::HashMap;

use common::network::{Zone as NetworkZone, Map as NetworkMap, MapLayer, Tile as NetworkTile};
use macroquad::prelude::*;
use mint::{Point2, Vector2};
use ndarray::Array2;
use strum::EnumCount;
use thiserror::Error;

use crate::ensure;

use super::{Zone, Map, Tile};

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
                contents.dim() == (other.width as usize, other.height as usize),
                MapError::IncorrectSize
            );
            layers.insert(layer, contents.map(|t| t.map(Into::into)));
            autotiles.insert(layer, Array2::default(contents.dim()));
        }

        let mut map = Self {
            id: other.id,
            width: other.width,
            height: other.height,
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
        let size = (other.width * other.height) as usize;
        assert_eq!(other.layers.len(), MapLayer::COUNT);

        let mut layers = HashMap::new();
        for (layer, contents) in other.layers {
            assert_eq!(contents.len(), size);
            layers.insert(layer, contents.map(|t| t.map(Into::into)));
        }

        Self {
            id: other.id,
            width: other.width,
            height: other.height,
            settings: other.settings,
            layers,
            zones: other.zones.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Tile> for NetworkTile {
    fn from(tile: Tile) -> Self {
        Self {
            texture: tile.texture.into(),
            autotile: tile.autotile,
            animation: tile.animation,
        }
    }
}

impl From<NetworkTile> for Tile {
    fn from(tile: NetworkTile) -> Self {
        Self {
            texture: tile.texture.into(),
            autotile: tile.autotile,
            animation: tile.animation,
        }
    }
}

impl From<NetworkZone> for Zone {
    fn from(other: NetworkZone) -> Self {
        Self {
            position: Rect::new(other.position.x, other.position.y, other.size.x, other.size.y),
            data: other.data,
        }
    }
}

impl From<Zone> for NetworkZone {
    fn from(other: Zone) -> Self {
        Self {
            position: Point2 {
                x: other.position.x,
                y: other.position.y,
            },
            size: Vector2 {
                x: other.position.w,
                y: other.position.h,
            },
            data: other.data,
        }
    }
}

use std::collections::HashMap;

use onyx_common::network::{MapLayer, Map as NetworkMap, Tile as NetworkTile, Area as NetworkArea};
use macroquad::prelude::*;
use mint::{Vector2, Point2};
use ndarray::Array2;
use strum::EnumCount;
use thiserror::Error;

use crate::map::Map;
use crate::ensure;

use super::{Tile, Area};

#[derive(Debug, Error)]
pub enum MapError {
    #[error("size is incorrect")]
    IncorrectSize,
    #[error("the number of layers is incorrect")]
    IncorrectLayers,
}

impl TryFrom<NetworkMap> for Map {
    type Error = MapError;

    fn try_from(value: NetworkMap) -> Result<Self, Self::Error> {
        let size = (value.width * value.height) as usize;
        ensure!(value.layers.len() == MapLayer::COUNT, MapError::IncorrectLayers);

        let mut layers = HashMap::new();
        let mut autotiles = HashMap::new();
        for (layer, contents) in value.layers {
            ensure!(contents.len() == size, MapError::IncorrectSize);
            layers.insert(layer, contents.map(|t| t.map(Into::into)));
            autotiles.insert(layer, Array2::default(contents.dim()));
        }

        let mut map = Self {
            width: value.width,
            height: value.height,
            layers, 
            autotiles,
            areas: value.areas.into_iter().map(Into::into).collect(),
        };

        map.update_autotile_cache();
        Ok(map)
    }
}

// Note: It is considered an unrecoverable error to have a map that has an invalid size
impl From<Map> for NetworkMap {
    fn from(value: Map) -> Self {
        let size = (value.width * value.height) as usize;
        assert_eq!(value.layers.len(), MapLayer::COUNT);

        let mut layers = HashMap::new();
        for (layer, contents) in value.layers {
            assert_eq!(contents.len(), size);
            layers.insert(layer, contents.map(|t| t.map(Into::into)));
        }

        Self {
            width: value.width,
            height: value.height,
            layers, 
            areas: value.areas.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Tile> for NetworkTile {
    fn from(tile: Tile) -> Self {
        Self {
            texture: tile.texture.into(),
            autotile: tile.autotile,
            animation: tile.animation
        }
    }
}

impl From<NetworkTile> for Tile {
    fn from(tile: NetworkTile) -> Self {
        Self {
            texture: tile.texture.into(),
            autotile: tile.autotile,
            animation: tile.animation
        }
    }
}

impl From<NetworkArea> for Area {
    fn from(other: NetworkArea) -> Self {
        Self {
            position: Rect::new(other.position.x, other.position.y, other.size.x, other.size.y),
            data: other.data,
        }
    }
}

impl From<Area> for NetworkArea {
    fn from(other: Area) -> Self {
        Self {
            position: Point2 { x: other.position.x, y: other.position.y },
            size: Vector2 { x: other.position.w, y: other.position.h },
            data: other.data
        }
    }
}
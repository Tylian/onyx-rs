use std::collections::HashMap;

use onyx_common::TILE_SIZE;
use onyx_common::network::{MapLayer, Map as NetworkMap, Tile as NetworkTile, Area as NetworkArea, AreaData};
use macroquad::prelude::*;
use mint::{Vector2, Point2};
use ndarray::Array2;
use thiserror::Error;

use crate::assets::Assets;
use crate::ensure;

const OFFSETS: &[(i32, i32)] = &[
    (0, -1), (1, 0), (0, 1), (-1, 0),
    (1, -1), (1, 1), (-1, 1), (-1, -1)
];

fn autotile_a(neighbors: u8) -> IVec2 {
    if neighbors == 0 { return ivec2(0, 0); };

    let neighbors = neighbors & (1 | 8 | 128);
    let (x, y) = match neighbors {
        128 | 0 => (0, 2),
        1 | 129 => (0, 4),
        8 | 136 => (2, 2),
        9 => (2, 0),
        137 => (2, 4),
        _ => unreachable!("autotile_a: {neighbors}")
    };
    ivec2(x, y)
}

fn autotile_b(neighbors: u8) -> IVec2 {
    if neighbors == 0 { return ivec2(1, 0); };

    let neighbors = neighbors & (1 | 2 | 16);
    let (x, y) = match neighbors {
        16 | 0 => (3, 2),
        1 | 17 => (3, 4),
        2 | 18 => (1, 2),
        3 => (3, 0),
        19 => (1, 4),
        _ => unreachable!("autotile_b: {neighbors}")
    };
    ivec2(x, y)
}

fn autotile_c(neighbors: u8) -> IVec2 {
    if neighbors == 0 { return ivec2(0, 1); };

    let neighbors = neighbors & (4 | 8 | 64);
    let (x, y) = match neighbors {
        64 | 0 => (0, 5),
        4 | 68 => (0, 3),
        8 | 72 => (2, 5),
        12 => (2, 1),
        76 => (2, 3),
        _ => unreachable!("autotile_c: {neighbors}")
    };
    ivec2(x, y)
}

fn autotile_d(neighbors: u8) -> IVec2 {
    if neighbors == 0 { return ivec2(1, 1); };
    let neighbors = neighbors & (4 | 2 | 32);
    let (x, y) = match neighbors {
        32 | 0 => (3, 5),
        4 | 36 => (3, 3),
        2 | 34 => (1, 5),
        6 => (3, 1),
        38 => (1, 3),
        _ => unreachable!("autotile_d: {neighbors}")
    };
    ivec2(x, y)
}

#[derive(Copy, Clone)]
pub enum Tile {
    Empty,
    Basic(IVec2),
    Autotile {
        base: IVec2,
        cache: [IVec2; 4],
    }
}

impl Default for Tile {
    fn default() -> Self {
        Tile::Empty
    }
}

impl Tile {
    pub fn empty() -> Self {
        Self::Empty
    }
    pub fn basic(uv: IVec2) -> Self {
        Self::Basic(uv)
    }
    pub fn autotile(uv: IVec2) -> Self {
        Self::Autotile {
            base: uv,
            cache: Default::default()
        }
    }

    fn get_uv(&self) -> Option<IVec2> {
        match *self {
            Tile::Empty => None,
            Tile::Basic(uv) => Some(uv),
            Tile::Autotile { base, .. } => Some(base),
        }
    }

    pub fn update_autotile(&mut self, neighbors: u8) {
        if let Self::Autotile { base, cache } = self {
            let base = *base * 2;
            *cache = [
                base + autotile_a(neighbors),
                base + autotile_b(neighbors),
                base + autotile_c(neighbors),
                base + autotile_d(neighbors),
            ];
        } 
    }

    pub fn draw(&self, position: Vec2, assets: &Assets) {
        const A: (f32, f32) = (0.0, 0.0);
        const B: (f32, f32) = (24.0, 0.0);
        const C: (f32, f32) = (0.0, 24.0);
        const D: (f32, f32) = (24.0, 24.0);

        match self {
            Tile::Empty => (),
            Tile::Basic(uv) => self.draw_tile(position, *uv, assets),
            Tile::Autotile { cache, .. } => {
                self.draw_subtile(position + A.into(), cache[0], assets);
                self.draw_subtile(position + B.into(), cache[1], assets);
                self.draw_subtile(position + C.into(), cache[2], assets);
                self.draw_subtile(position + D.into(), cache[3], assets);
            },
        }
    }

    fn draw_tile(&self, position: Vec2, uv: IVec2, assets: &Assets) {
        let uv = uv * TILE_SIZE;
        let source = Rect::new(uv.x as f32, uv.y as f32, TILE_SIZE as f32, TILE_SIZE as f32);

        draw_texture_ex(
            assets.tileset,
            position.x,
            position.y,
            WHITE,
            DrawTextureParams {
                source: Some(source),
                ..Default::default()
            }
        );
    }

    fn draw_subtile(&self, position: Vec2, uv: IVec2, assets: &Assets) {
        let uv = uv * 24;
        let source = Rect::new(uv.x as f32, uv.y as f32, 24.0, 24.0);

        draw_texture_ex(
            assets.tileset,
            position.x,
            position.y,
            WHITE,
            DrawTextureParams {
                source: Some(source),
                ..Default::default()
            }
        );
    }
}

trait AttributeDataEx {
    fn text(&self) -> String;
    fn color(&self) -> Color;
}

impl AttributeDataEx for AreaData {
    fn text(&self) -> String {
        match self {
            AreaData::Blocked => String::from("Blocked"),
            AreaData::Log(message) => format!("Log: {}", message),
        }
    }
    fn color(&self) -> Color {
        match self {
            AreaData::Blocked => RED,
            AreaData::Log(_) => SKYBLUE,
        }
    }
}

pub fn draw_area(position: Rect, data: &AreaData, assets: &Assets) {
    let color = data.color();
    let text = data.text();

    let Rect { x, y, w, h } = position;
    draw_rectangle(x, y, w, h, Color::new(color.r, color.g, color.b, 0.4));
    draw_rectangle_lines(x, y, w, h, 1.0, color);

    let bounds = measure_text(&text, Some(assets.font), 16, 1.0);
    
    let text_x = x + (w - bounds.width) / 2.0;
    let text_y = y + (h - bounds.height) / 2.0 + bounds.offset_y;

    draw_text_ex(&text, text_x, text_y, TextParams {
        font: assets.font,
        font_size: 16,
        color,
        ..Default::default()
    });
}

#[derive(Clone, Debug)]
pub struct Area {
    pub position: Rect,
    pub data: AreaData,
}

impl Area {
    pub fn draw(&self, assets: &Assets) {
        draw_area(self.position, &self.data, assets);
    }
}

#[derive(Clone)]
pub struct Map {
    pub width: u32,
    pub height: u32,
    layers: HashMap<MapLayer, Array2<Tile>>,
    pub areas: Vec<Area>,
}

impl Map {
    pub fn new(width: u32, height: u32) -> Self {
        let mut layers = HashMap::new();

        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((width as usize, height as usize)));
        }

        Self {
            width,
            height,
            layers,
            areas: Vec::new(),
        }
    }

    pub fn pixel_size(&self) -> (f32, f32) {
        (self.width as f32 * TILE_SIZE as f32, self.height as f32 * TILE_SIZE as f32)
    }

    pub fn valid(&self, pos: IVec2) -> bool {
        pos.x >= 0 && pos.x < self.width as i32 && pos.y >= 0 && pos.y < self.height as i32
    }

    pub fn tile(&self, layer: MapLayer, position: IVec2) -> Option<&Tile> {
        self.layers[&layer].get((position.x as usize, position.y as usize))
    }

    pub fn tile_mut(&mut self, layer: MapLayer, position: IVec2) -> Option<&mut Tile> {
        self.layers.get_mut(&layer).unwrap().get_mut((position.x as usize, position.y as usize))
    }

    pub fn tiles(&self, layer: MapLayer) -> impl Iterator<Item = &Tile> {
        self.layers[&layer].iter()
    }

    pub fn draw_layer(&self, layer: MapLayer, assets: &Assets) {
        for (x, y) in itertools::iproduct!(0..self.width, 0..self.height) {
            let position = ivec2(x as i32, y as i32);
            let screen_position = position.as_f32() * TILE_SIZE as f32;
            if let Some(tile) = self.tile(layer, position) {
                tile.draw(screen_position, assets);
            }
        }
    }

    pub fn draw_areas(&self, assets: &Assets) {
        for attrib in &self.areas {
            attrib.draw(assets);
        }
    }

    pub fn update_autotiles(&mut self) {
        // collecting all the data i need because we can't borrow self in the loop lmao
        let ground_map = self.layers[&MapLayer::Ground].map(Tile::get_uv);
        let mask_map= self.layers[&MapLayer::Mask].map(Tile::get_uv);
        let fringe_map = self.layers[&MapLayer::Fringe].map(Tile::get_uv);

        for (x, y) in itertools::iproduct!(0..self.width, 0..self.height) {
            let position = ivec2(x as i32, y as i32);
            for layer in MapLayer::iter() {
                let neighbor_map = match layer {
                    MapLayer::Ground => &ground_map,
                    MapLayer::Mask => &mask_map,
                    MapLayer::Fringe => &fringe_map,
                };

                if let Some(tile) = self.tile_mut(layer, position) {
                    let uv = tile.get_uv();
                    let mut neighbors = 0;
                    for (i, offset) in OFFSETS.iter().enumerate() {
                        let neighbor = position + IVec2::from(*offset);
                        let neighbor = neighbor_map.get((neighbor.x as usize, neighbor.y as usize));

                        if let Some(neighbor) = neighbor {
                            match uv.zip(neighbor.as_ref()) {
                                Some((a, &b)) if a == b => { neighbors |= 1 << i; },
                                _ => ()
                            }
                        } else { // Auto-tiles look better when out of map tiles are assumed to be the same
                            neighbors |= 1 << i;
                        }
                    }
                    tile.update_autotile(neighbors);
                }
            }
            
        }
    }

    pub fn resize(&self, width: u32, height: u32) -> Map {
        let mut layers = HashMap::new();
        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((width as usize, height as usize)));
        }

        for (x, y) in itertools::iproduct!(0..self.width, 0..self.height) {
            let index = (x as usize, y as usize);
            for layer in MapLayer::iter() {
                if let Some(&tile) = self.tile(layer, ivec2(x as i32, y as i32)) {
                    if let Some(new_tile) = layers.get_mut(&layer).unwrap().get_mut(index) {
                        *new_tile = tile;
                    }
                }
            }
        }

        let map_rect = Rect::new(0., 0., width as f32 * TILE_SIZE as f32, height as f32 * TILE_SIZE as f32);
        let attributes = self.areas.iter().cloned()
            .filter(|attrib| map_rect.overlaps(&attrib.position))
            .collect();

        Map { width, height, layers, areas: attributes }
    }
}

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
        ensure!(value.layers.len() == MapLayer::count(), MapError::IncorrectLayers);

        let mut layers = HashMap::new();
        for (layer, contents) in value.layers {
            ensure!(contents.len() == size, MapError::IncorrectSize);
            layers.insert(layer, contents.map(|&t| t.into()));
        }

        let mut map = Self {
            width: value.width,
            height: value.height,
            layers, 
            areas: value.areas.into_iter().map(Into::into).collect(),
        };

        map.update_autotiles();

        Ok(map)
    }
}

impl From<NetworkTile> for Tile {
    fn from(remote: NetworkTile) -> Self {
        match remote {
            NetworkTile::Empty => Tile::Empty,
            NetworkTile::Basic(uv) => Tile::Basic(uv.into()),
            NetworkTile::Autotile(uv) => Tile::Autotile { base: uv.into(), cache: Default::default() },
        }
    }
}

// Note: It is considered an unrecoverable error to have a map that has an invalid size
impl From<Map> for NetworkMap {
    fn from(value: Map) -> Self {
        let size = (value.width * value.height) as usize;
        assert_eq!(value.layers.len(), MapLayer::count());

        let mut layers = HashMap::new();
        for (layer, contents) in value.layers {
            assert_eq!(contents.len(), size);
            layers.insert(layer, contents.map(|&t| t.into()));
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
        match tile {
            Tile::Empty => NetworkTile::Empty,
            Tile::Basic(uv) => NetworkTile::Basic(uv.into()),
            Tile::Autotile { base, .. } => NetworkTile::Autotile(base.into()),
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
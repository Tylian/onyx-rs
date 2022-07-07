use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use common::network::{Map as NetworkMap, MapHash, MapLayer, MapSettings, TileAnimation, ZoneData};
use common::TILE_SIZE;
use macroquad::prelude::*;
use ndarray::{azip, indices, Array2, Zip};
use strum::{EnumCount, IntoEnumIterator};

use crate::assets::Assets;
use crate::utils::draw_text_shadow;
use crate::utils::ping_pong;

mod interop;

const OFFSETS: &[(i32, i32)] = &[(0, -1), (1, 0), (0, 1), (-1, 0), (1, -1), (1, 1), (-1, 1), (-1, -1)];

fn autotile_a(neighbors: u8) -> IVec2 {
    if neighbors == 0 {
        return ivec2(0, 0);
    };

    let neighbors = neighbors & (1 | 8 | 128);
    let (x, y) = match neighbors {
        128 | 0 => (0, 2),
        1 | 129 => (0, 4),
        8 | 136 => (2, 2),
        9 => (2, 0),
        137 => (2, 4),
        _ => unreachable!("autotile_a: {neighbors}"),
    };
    ivec2(x, y)
}

fn autotile_b(neighbors: u8) -> IVec2 {
    if neighbors == 0 {
        return ivec2(1, 0);
    };

    let neighbors = neighbors & (1 | 2 | 16);
    let (x, y) = match neighbors {
        16 | 0 => (3, 2),
        1 | 17 => (3, 4),
        2 | 18 => (1, 2),
        3 => (3, 0),
        19 => (1, 4),
        _ => unreachable!("autotile_b: {neighbors}"),
    };
    ivec2(x, y)
}

fn autotile_c(neighbors: u8) -> IVec2 {
    if neighbors == 0 {
        return ivec2(0, 1);
    };

    let neighbors = neighbors & (4 | 8 | 64);
    let (x, y) = match neighbors {
        64 | 0 => (0, 5),
        4 | 68 => (0, 3),
        8 | 72 => (2, 5),
        12 => (2, 1),
        76 => (2, 3),
        _ => unreachable!("autotile_c: {neighbors}"),
    };
    ivec2(x, y)
}

fn autotile_d(neighbors: u8) -> IVec2 {
    if neighbors == 0 {
        return ivec2(1, 1);
    };
    let neighbors = neighbors & (4 | 2 | 32);
    let (x, y) = match neighbors {
        32 | 0 => (3, 5),
        4 | 36 => (3, 3),
        2 | 34 => (1, 5),
        6 => (3, 1),
        38 => (1, 3),
        _ => unreachable!("autotile_d: {neighbors}"),
    };
    ivec2(x, y)
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Tile {
    pub texture: IVec2,
    pub autotile: bool,
    pub animation: Option<TileAnimation>,
}

impl Tile {
    pub fn draw(&self, position: Vec2, time: f64, assets: &Assets) {
        let mut uv = self.texture * TILE_SIZE;

        if let Some(animation) = self.animation {
            let t = (time % animation.duration) / animation.duration;

            if animation.bouncy {
                uv.x += ping_pong(t, animation.frames) as i32 * TILE_SIZE;
            } else {
                uv.x += (t * animation.frames as f64) as i32 * TILE_SIZE;
            }
        }

        let source = Rect::new(uv.x as f32, uv.y as f32, TILE_SIZE as f32, TILE_SIZE as f32);
        draw_texture_ex(
            assets.tileset().texture,
            position.x,
            position.y,
            WHITE,
            DrawTextureParams {
                source: Some(source),
                ..Default::default()
            },
        );
    }
}

#[derive(Clone, Debug, Default)]
#[repr(transparent)]
struct AutoTile {
    cache: [IVec2; 4],
}

impl AutoTile {
    pub fn new(base: IVec2, neighbors: u8) -> Self {
        Self {
            cache: [
                base + autotile_a(neighbors),
                base + autotile_b(neighbors),
                base + autotile_c(neighbors),
                base + autotile_d(neighbors),
            ],
        }
    }
    pub fn draw(&self, position: Vec2, animation: Option<TileAnimation>, time: f64, assets: &Assets) {
        const A: (f32, f32) = (0.0, 0.0);
        const B: (f32, f32) = (24.0, 0.0);
        const C: (f32, f32) = (0.0, 24.0);
        const D: (f32, f32) = (24.0, 24.0);

        let draw_subtile = |position: Vec2, uv: IVec2| {
            let mut uv = uv * TILE_SIZE / 2;

            if let Some(animation) = animation {
                let t = (time % animation.duration) / animation.duration;

                if animation.bouncy {
                    uv.x += ping_pong(t, animation.frames) as i32 * (TILE_SIZE * 2);
                } else {
                    uv.x += (t * animation.frames as f64) as i32 * (TILE_SIZE * 2);
                }
            }

            let source = Rect::new(uv.x as f32, uv.y as f32, 24.0, 24.0);
            draw_texture_ex(
                assets.tileset().texture,
                position.x,
                position.y,
                WHITE,
                DrawTextureParams {
                    source: Some(source),
                    ..Default::default()
                },
            );
        };

        draw_subtile(position + A.into(), self.cache[0]);
        draw_subtile(position + B.into(), self.cache[1]);
        draw_subtile(position + C.into(), self.cache[2]);
        draw_subtile(position + D.into(), self.cache[3]);
    }
}

trait AttributeDataEx {
    fn text(&self) -> String;
    fn color(&self) -> Color;
}

impl AttributeDataEx for ZoneData {
    fn text(&self) -> String {
        match self {
            ZoneData::Blocked => String::from("Blocked"),
            ZoneData::Warp(_, _, _) => String::from("Warp"),
        }
    }
    fn color(&self) -> Color {
        match self {
            ZoneData::Blocked => RED,
            ZoneData::Warp(_, _, _) => GREEN,
        }
    }
}

pub fn draw_zone(position: Rect, data: &ZoneData, assets: &Assets) {
    let color = data.color();
    let text = data.text();

    let Rect { x, y, w, h } = position;
    draw_rectangle(x, y, w, h, Color::new(color.r, color.g, color.b, 0.4));
    draw_rectangle_lines(x, y, w, h, 1.0, color);

    let bounds = measure_text(&text, Some(assets.font), 16, 1.0);
    let text_pos = vec2(
        x + (w - bounds.width) / 2.0,
        y + (h - bounds.height) / 2.0 + bounds.offset_y,
    );

    draw_text_shadow(
        &text,
        text_pos,
        TextParams {
            font: assets.font,
            font_size: 16,
            color,
            ..Default::default()
        },
    );
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub position: Rect,
    pub data: ZoneData,
}

impl Zone {
    pub fn draw(&self, assets: &Assets) {
        draw_zone(self.position, &self.data, assets);
    }
}

#[derive(Clone)]
pub struct Map {
    pub id: String,
    pub hash: MapHash,
    pub width: u32,
    pub height: u32,
    pub settings: MapSettings,
    layers: HashMap<MapLayer, Array2<Option<Tile>>>,
    autotiles: HashMap<MapLayer, Array2<Option<AutoTile>>>,
    pub zones: Vec<Zone>,
}

impl Map {
    pub fn cache_path(hash: MapHash) -> PathBuf {
        let mut path = common::client_runtime!();
        path.push("maps");
        path.push(format!("{:x}.bin", hash.0));
        path
    }

    pub fn from_cache(hash: MapHash) -> Result<Self> {
        let path = Self::cache_path(hash);
        let file = std::fs::File::open(path)?;
        let map: NetworkMap = rmp_serde::from_read(file)?;

        Ok(map.try_into()?)
    }

    pub fn save_cache(&self) -> Result<()> {
        let map = NetworkMap::from(self.clone());
        let bytes = rmp_serde::to_vec_named(&map)?;
        let path = Self::cache_path(self.hash);
        std::fs::write(path, bytes)?;

        Ok(())
    }

    pub fn new(id: &str, width: u32, height: u32) -> Self {
        let settings = MapSettings::default();
        let mut layers = HashMap::new();
        let mut autotiles = HashMap::new();
        let zones = Vec::new();
        let hash = MapHash::from(id);

        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((width as usize, height as usize)));
            autotiles.insert(layer, Array2::default((width as usize, height as usize)));
        }

        Self {
            id: id.to_string(),
            hash,
            width,
            height,
            settings,
            layers,
            autotiles,
            zones,
        }
    }

    pub fn pixel_size(&self) -> (f32, f32) {
        (
            self.width as f32 * TILE_SIZE as f32,
            self.height as f32 * TILE_SIZE as f32,
        )
    }

    pub fn valid(&self, pos: IVec2) -> bool {
        pos.x >= 0 && pos.x < self.width as i32 && pos.y >= 0 && pos.y < self.height as i32
    }

    pub fn fill(&mut self, layer: MapLayer, tile: Option<Tile>) {
        self.layers.get_mut(&layer).unwrap().fill(tile);
    }

    pub fn tile(&self, layer: MapLayer, position: IVec2) -> Option<&Tile> {
        self.layers[&layer]
            .get((position.x as usize, position.y as usize))
            .and_then(Option::as_ref)
    }

    pub fn tile_mut(&mut self, layer: MapLayer, position: IVec2) -> Option<&mut Tile> {
        self.layers
            .get_mut(&layer)
            .unwrap()
            .get_mut((position.x as usize, position.y as usize))
            .and_then(Option::as_mut)
    }

    // Sets a tile, returning the previous one if it existed
    pub fn set_tile(&mut self, layer: MapLayer, position: IVec2, tile: Tile) -> Option<Tile> {
        self.layers
            .get_mut(&layer)
            .unwrap()
            .get_mut((position.x as usize, position.y as usize))
            .and_then(|inner| inner.replace(tile))
    }

    // Clears the tile, returning it if there was one
    pub fn clear_tile(&mut self, layer: MapLayer, position: IVec2) -> Option<Tile> {
        self.layers
            .get_mut(&layer)
            .unwrap()
            .get_mut((position.x as usize, position.y as usize))
            .and_then(Option::take)
    }

    pub fn tiles(&self, layer: MapLayer) -> impl Iterator<Item = Option<&Tile>> {
        self.layers[&layer].iter().map(Option::as_ref)
    }

    pub fn draw_layer(&self, layer: MapLayer, time: f64, assets: &Assets) {
        let layers = &self.layers[&layer];
        let autotiles = &self.autotiles[&layer];
        azip!((index (x, y), tile in layers, autotile in autotiles) {
            let position = ivec2(x as i32, y as i32);
            let screen_position = position.as_f32() * TILE_SIZE as f32;
            if let Some(autotile) = autotile {
                autotile.draw(screen_position, tile.and_then(|t| t.animation), time, assets);
            } else if let Some(tile) = tile {
                tile.draw(screen_position, time, assets);
            }
        });
    }

    pub fn draw_zones(&self, assets: &Assets) {
        for attrib in &self.zones {
            attrib.draw(assets);
        }
    }

    pub fn update_autotile_cache(&mut self) {
        for layer in MapLayer::iter() {
            let texture_map = self.layers[&layer].map(|tile| match tile {
                Some(tile) if tile.autotile => Some(tile.texture),
                _ => None,
            });

            let neighbor_map = Zip::indexed(&texture_map).map_collect(|index, texture| {
                if let &Some(texture) = texture {
                    let position = IVec2::new(index.0 as i32, index.1 as i32);
                    let mut neighbors = 0;
                    for (i, offset) in OFFSETS.iter().enumerate() {
                        let neighbor = position + IVec2::from(*offset);
                        let neighbor = texture_map.get((neighbor.x as usize, neighbor.y as usize));

                        if let Some(neighbor) = neighbor {
                            match (texture, *neighbor) {
                                (a, Some(b)) if a == b => {
                                    neighbors |= 1 << i;
                                }
                                _ => (),
                            }
                        } else {
                            // Auto-tiles look better when out of map tiles are assumed to be the same
                            neighbors |= 1 << i;
                        }
                    }

                    Some((texture, neighbors))
                } else {
                    None
                }
            });

            let autotile_cache = neighbor_map.map(|info| {
                info.map(|(texture, neighbors)| {
                    let base = texture * 2;
                    AutoTile::new(base, neighbors)
                })
            });

            self.autotiles.insert(layer, autotile_cache);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let dimensions = (width as usize, height as usize);
        let mut layers = HashMap::with_capacity(MapLayer::COUNT);

        for layer in MapLayer::iter() {
            let tiles = Zip::from(indices(dimensions)).map_collect(|index| self.layers[&layer][index]);
            layers.insert(layer, tiles);
        }

        let map_rect = Rect::new(
            0.0,
            0.0,
            width as f32 * TILE_SIZE as f32,
            height as f32 * TILE_SIZE as f32,
        );

        self.zones.retain(|zone| map_rect.overlaps(&zone.position));

        self.width = width;
        self.height = height;
        self.layers = layers;

        self.update_autotile_cache();
    }
}

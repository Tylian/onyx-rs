use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use ggez::glam::*;
use ggez::graphics::{Canvas, Color, DrawParam, InstanceArray};
use ggez::{Context, GameResult};
use ndarray::{azip, indices, Array2, Zip};
use onyx::math::units::map::*;
use onyx::math::units::world;
use onyx::network::{
    Map as NetworkMap,
    MapId,
    MapLayer,
    MapSettings,
    Tile as NetworkTile,
    TileAnimation,
    Zone,
    ZoneData,
};
use onyx::TILE_SIZE;
use strum::{EnumCount, IntoEnumIterator};
use thiserror::Error;

use crate::utils::{ping_pong, OutlinedText};
use crate::{ensure, AssetCache};

const OFFSETS: &[(i32, i32)] = &[(0, -1), (1, 0), (0, 1), (-1, 0), (1, -1), (1, 1), (-1, 1), (-1, -1)];

fn autotile_a(neighbors: u8) -> UVec2 {
    if neighbors == 0 {
        return uvec2(0, 0);
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
    uvec2(x, y)
}

fn autotile_b(neighbors: u8) -> UVec2 {
    if neighbors == 0 {
        return uvec2(1, 0);
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
    uvec2(x, y)
}

fn autotile_c(neighbors: u8) -> UVec2 {
    if neighbors == 0 {
        return uvec2(0, 1);
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
    uvec2(x, y)
}

fn autotile_d(neighbors: u8) -> UVec2 {
    if neighbors == 0 {
        return uvec2(1, 1);
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
    uvec2(x, y)
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Tile {
    pub texture: UVec2,
    pub autotile: bool,
    pub animation: Option<TileAnimation>,
}

impl Tile {
    pub fn draw(&self, batch: &mut InstanceArray, position: world::Point2D, assets: &mut AssetCache, time: f32) {
        let mut uv = self.texture * TILE_SIZE as u32;

        if let Some(animation) = self.animation {
            let t = (time % animation.duration) / animation.duration;

            if animation.bouncy {
                uv.x += ping_pong(t, animation.frames) * TILE_SIZE as u32;
            } else {
                uv.x += (animation.frames as f32 * t) as u32 * TILE_SIZE as u32;
            }
        }

        let uv = assets.tileset.uv_rect(uv.x, uv.y, TILE_SIZE as u32, TILE_SIZE as u32);
        batch.push(DrawParam::default().src(uv).dest(position));
    }
}

#[derive(Clone, Debug, Default)]
#[repr(transparent)]
struct AutoTile {
    cache: [UVec2; 4],
}

impl AutoTile {
    pub fn new(base: UVec2, neighbors: u8) -> Self {
        Self {
            cache: [
                base + autotile_a(neighbors),
                base + autotile_b(neighbors),
                base + autotile_c(neighbors),
                base + autotile_d(neighbors),
            ],
        }
    }
    pub fn draw(
        &self,
        batch: &mut InstanceArray,
        position: world::Point2D,
        animation: Option<TileAnimation>,
        assets: &mut AssetCache,
        time: f32,
    ) {
        const A: world::Vector2D = world::Vector2D::new(0.0, 0.0);
        const B: world::Vector2D = world::Vector2D::new(24.0, 0.0);
        const C: world::Vector2D = world::Vector2D::new(0.0, 24.0);
        const D: world::Vector2D = world::Vector2D::new(24.0, 24.0);

        let draw_subtile = |batch: &mut InstanceArray, position: world::Point2D, uv: UVec2| {
            let mut uv = uv * (TILE_SIZE as u32) / 2;

            if let Some(animation) = animation {
                let t = (time % animation.duration) / animation.duration;

                if animation.bouncy {
                    uv.x += ping_pong(t, animation.frames) * ((TILE_SIZE as u32) * 2);
                } else {
                    uv.x += (t * animation.frames as f32) as u32 * ((TILE_SIZE as u32) * 2);
                }
            }

            let uv = assets.tileset.uv_rect(
                uv.x,
                uv.y,
                ((TILE_SIZE as i32) / 2) as u32,
                ((TILE_SIZE as i32) / 2) as u32,
            );

            batch.push(DrawParam::default().src(uv).dest(position));
        };

        draw_subtile(batch, position + A, self.cache[0]);
        draw_subtile(batch, position + B, self.cache[1]);
        draw_subtile(batch, position + C, self.cache[2]);
        draw_subtile(batch, position + D, self.cache[3]);
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
            ZoneData::Blocked => Color::RED,
            ZoneData::Warp(_, _, _) => Color::GREEN,
        }
    }
}

#[allow(unused)]
pub fn draw_zone(ctx: &mut Context, canvas: &mut Canvas, position: world::Box2D, data: &ZoneData) -> GameResult {
    use ggez::graphics::*;

    let color = data.color();
    let background_color = Color::new(color.r, color.g, color.b, 0.4);

    // todo ggez::graphics::Mesh::new_rectangle
    let rect = Rect::new(position.min.x, position.min.y, position.width(), position.height());

    let background = ggez::graphics::Mesh::new_rectangle(ctx, DrawMode::fill(), rect, background_color)?;

    let outline = ggez::graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), rect, color)?;

    canvas.draw(&background, DrawParam::default());
    canvas.draw(&outline, DrawParam::default());

    let mut text = Text::new(data.text());
    text.set_layout(TextLayout::center());
    text.set_scale(PxScale::from(16.0));

    canvas.draw(
        &OutlinedText::new(&text),
        DrawParam::default().dest(position.center()).color(color),
    );

    Ok(())
}

#[derive(Clone)]
pub struct Map {
    pub id: MapId,
    pub size: Size2D,
    pub settings: MapSettings,
    layers: HashMap<MapLayer, Array2<Option<Tile>>>,
    autotiles: HashMap<MapLayer, Array2<Option<AutoTile>>>,
    pub zones: Vec<Zone>,
}

impl Map {
    pub fn cache_path(id: MapId) -> PathBuf {
        PathBuf::from(format!("maps/{}.bin", id.0))
    }

    pub fn from_cache(id: MapId) -> Result<Self> {
        let path = Self::cache_path(id);
        let file = std::fs::File::open(path)?;
        let map: NetworkMap = rmp_serde::from_read(file)?;

        Ok(map.try_into()?)
    }

    pub fn save_cache(&self) -> Result<()> {
        let map = NetworkMap::from(self.clone());
        let bytes = rmp_serde::to_vec_named(&map)?;
        let path = Self::cache_path(self.id);
        std::fs::write(path, bytes)?;

        Ok(())
    }

    pub fn new(id: MapId, size: Size2D) -> Self {
        let settings = MapSettings::default();
        let mut layers = HashMap::new();
        let mut autotiles = HashMap::new();
        let zones = Vec::new();

        for layer in MapLayer::iter() {
            layers.insert(layer, Array2::default((size.width as usize, size.height as usize)));
            autotiles.insert(layer, Array2::default((size.width as usize, size.height as usize)));
        }

        Self {
            id,
            size,
            settings,
            layers,
            autotiles,
            zones,
        }
    }

    pub fn world_size(&self) -> world::Size2D {
        (self.size.to_f32() * TILE_SIZE).cast_unit()
    }

    pub fn valid(&self, pos: Point2D) -> bool {
        Box2D::from_size(self.size).contains(pos)
    }

    pub fn fill(&mut self, layer: MapLayer, tile: Option<Tile>) {
        self.layers.get_mut(&layer).unwrap().fill(tile);
    }

    pub fn tile(&self, layer: MapLayer, position: Point2D) -> Option<&Tile> {
        self.layers[&layer]
            .get((position.x as usize, position.y as usize))
            .and_then(Option::as_ref)
    }

    pub fn tile_mut(&mut self, layer: MapLayer, position: Point2D) -> Option<&mut Tile> {
        self.layers
            .get_mut(&layer)
            .unwrap()
            .get_mut((position.x as usize, position.y as usize))
            .and_then(Option::as_mut)
    }

    // Sets a tile, returning the previous one if it existed
    pub fn set_tile(&mut self, layer: MapLayer, position: Point2D, tile: Tile) -> Option<Tile> {
        self.layers
            .get_mut(&layer)
            .unwrap()
            .get_mut((position.x as usize, position.y as usize))
            .and_then(|inner| inner.replace(tile))
    }

    // Clears the tile, returning it if there was one
    pub fn clear_tile(&mut self, layer: MapLayer, position: Point2D) -> Option<Tile> {
        self.layers
            .get_mut(&layer)
            .unwrap()
            .get_mut((position.x as usize, position.y as usize))
            .and_then(Option::take)
    }

    pub fn tiles(&self, layer: MapLayer) -> impl Iterator<Item = Option<&Tile>> {
        self.layers[&layer].iter().map(Option::as_ref)
    }

    pub fn draw_layer(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        layer: MapLayer,
        assets: &mut AssetCache,
        time: f32,
    ) {
        let mut batch = InstanceArray::new(ctx, Some(assets.tileset.clone()));

        let layers = &self.layers[&layer];
        let autotiles = &self.autotiles[&layer];
        azip!((index (x, y), tile in layers, autotile in autotiles) {
            let position = world::Point2D::new(x as f32, y as f32) * TILE_SIZE;
            if let Some(autotile) = autotile {
                autotile.draw(&mut batch, position, tile.and_then(|t| t.animation), assets, time);
            } else if let Some(tile) = tile {
                tile.draw(&mut batch, position, assets, time);
            }
        });

        canvas.draw(&batch, DrawParam::default());
    }

    // why yes i am lazy
    pub fn draw_layers(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        layers: &[MapLayer],
        assets: &mut AssetCache,
        time: f32,
    ) {
        for layer in layers.iter() {
            self.draw_layer(ctx, canvas, *layer, assets, time);
        }
    }

    pub fn draw_zones(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        for attrib in &self.zones {
            draw_zone(ctx, canvas, attrib.position, &attrib.data)?
        }

        Ok(())
    }

    pub fn update_autotile_cache(&mut self) {
        use onyx::math::units::map::{Point2D, Vector2D};

        for layer in MapLayer::iter() {
            let texture_map = self.layers[&layer].map(|tile| match tile {
                Some(tile) if tile.autotile => Some(tile.texture),
                _ => None,
            });

            let neighbor_map = Zip::indexed(&texture_map).map_collect(|index, texture| {
                if let &Some(texture) = texture {
                    let position = Point2D::new(index.0 as i32, index.1 as i32);
                    let mut neighbors = 0;
                    for (i, offset) in OFFSETS.iter().enumerate() {
                        let neighbor = position + Vector2D::from(*offset);
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

    pub fn resize(&mut self, size: Size2D) {
        use euclid::Scale;

        let dimensions = (size.width as usize, size.height as usize);
        let mut layers = HashMap::with_capacity(MapLayer::COUNT);

        for layer in MapLayer::iter() {
            let tiles = Zip::from(indices(dimensions)).map_collect(|index| self.layers[&layer][index]);
            layers.insert(layer, tiles);
        }

        let map_size = size.to_f32() * Scale::new(TILE_SIZE);

        let map_rect = world::Box2D::from_size(map_size);

        self.zones.retain(|zone| map_rect.contains(zone.position.min));

        self.size = size;
        self.layers = layers;

        self.update_autotile_cache();
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

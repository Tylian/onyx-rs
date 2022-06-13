use crate::prelude::*;
use crate::game::Assets;

#[derive(Copy, Clone, Default)]
pub struct Tile {
    pub index: u32,
}

pub struct Map {
    width: u32,
    height: u32,
    tiles: Vec<Tile>
}

impl Map {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height).try_into().unwrap();
        let tiles = vec![Tile::default(); size];
        Self {
            width,
            height,
            tiles
        }
    }

    pub fn tile(&self, x: u32, y: u32) -> Option<&Tile> {
        if x > self.width || y > self.height {
            return None;
        }

        // usize shouldn't lose precision because the width and height were checked in Self::new 
        self.tiles.get((x + y * self.width) as usize)
    }

    pub fn tile_mut(&mut self, x: u32, y: u32) -> Option<&mut Tile> {
        if x > self.width || y > self.height {
            return None;
        }

        // usize shouldn't lose precision because the width and height were checked in Self::new 
        self.tiles.get_mut((x + y * self.width) as usize)
    }

    pub fn draw(&self, assets: &Assets) {
        for (i, tile) in self.tiles.iter().enumerate() {
            let i: u32 = i.try_into().unwrap();
            let x = i % self.width;
            let y = i / self.width;

            let source = Rect::new(
                (tile.index % 16) as f32 * 48.0,
                (tile.index / 16) as f32 * 48.0,
                48.0,
                48.0
            );

            draw_texture_ex(
                assets.tileset,
                x as f32 * 48.0,
                y as f32 * 48.0,
                WHITE,
                DrawTextureParams {
                    source: Some(source),
                    ..Default::default()
                }
            );
        }
    }

    pub fn valid(&self, pos: Vec2) -> bool {
        pos.x >= 0.0 && (pos.x as u32) < self.width && pos.y >= 0.0 && (pos.y as u32) < self.height
    }
}
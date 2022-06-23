use macroquad::prelude::*;
use onyx_common::{
    network::{ClientId, Direction, PlayerData},
    TILE_SIZE,
};

use crate::{
    assets::Assets,
    utils::{draw_text_shadow, ping_pong},
};

use super::SPRITE_SIZE;

pub enum Animation {
    Standing,
    Walking {
        /// Start time of the animation
        start: f64,
        /// Movement speed in pixels per second.
        speed: f64,
    },
}

impl Animation {
    fn get_animation_offset(&self, time: f64, direction: Direction) -> Vec2 {
        let offset_y = match direction {
            Direction::South => 0.0,
            Direction::West => 1.0,
            Direction::East => 2.0,
            Direction::North => 3.0,
        };

        let offset_x = match self {
            Animation::Standing => 1.0,
            Animation::Walking { start, speed } => {
                let length = 2.0 * TILE_SIZE as f64 / speed;
                ping_pong(((time - start) / length) % 1.0, 3) as f32
            }
        };

        vec2(offset_x * SPRITE_SIZE as f32, offset_y * SPRITE_SIZE as f32)
    }
}

#[derive(Copy, Clone)]
pub struct Tween {
    pub velocity: Vec2,
    pub last_update: f64,
}

pub struct Player {
    pub id: ClientId,
    pub name: String,
    pub position: Vec2,
    pub tween: Option<Tween>,
    pub animation: Animation,
    pub sprite: u32,
    pub direction: Direction,
}

impl Player {
    pub fn from_network(id: ClientId, data: PlayerData) -> Self {
        Self {
            id,
            name: data.name,
            position: data.position.into(),
            animation: Animation::Standing,
            tween: None,
            sprite: data.sprite,
            direction: Direction::South,
        }
    }

    pub fn draw(&self, time: f64, assets: &Assets) {
        self.draw_text(assets, self.position);
        self.draw_sprite(assets, self.position, time);
    }

    pub fn draw_text(&self, assets: &Assets, position: Vec2) {
        const FONT_SIZE: u16 = 16;
        let measurements = measure_text(&self.name, Some(assets.font), FONT_SIZE, 1.0);

        // ? The text is drawn with the baseline being the supplied y
        let text_offset = ((SPRITE_SIZE as f32 - measurements.width) / 2.0, -3.0).into();

        let pos = position + text_offset;
        draw_text_shadow(
            &self.name,
            pos,
            TextParams {
                font_size: FONT_SIZE,
                font: assets.font,
                color: WHITE,
                ..Default::default()
            },
        );
    }
    fn draw_sprite(&self, assets: &Assets, position: Vec2, time: f64) {
        let offset = self.animation.get_animation_offset(time, self.direction);

        let sprite_x = (self.sprite as f32 % 4.0) * 3.0;
        let sprite_y = (self.sprite as f32 / 4.0).floor() * 4.0;

        let source = Rect::new(
            sprite_x * SPRITE_SIZE as f32 + offset.x,
            sprite_y * SPRITE_SIZE as f32 + offset.y,
            SPRITE_SIZE as f32,
            SPRITE_SIZE as f32,
        );

        draw_texture_ex(
            assets.sprites.texture,
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

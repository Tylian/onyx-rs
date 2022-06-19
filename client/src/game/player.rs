use onyx_common::network::{ClientId, Direction, PlayerData};
use macroquad::prelude::*;

use crate::assets::Assets;

use super::SPRITE_SIZE;

pub enum Animation {
    Standing,
    Walking {
        start: f64,
        // speed in pixels per second
        speed: f64
    }
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
            Animation::Standing => {
                1.0
            },
            Animation::Walking { start, speed } => {
                // speed is set so that you take a step every 24 pixels, or half a tile.
                let t = ((time - start) * speed / 24.) as f32;
                ((t % 4.0).floor() - 1.).abs()
            },
        };

        vec2(offset_x * SPRITE_SIZE, offset_y * SPRITE_SIZE)
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
            position: data.position.into(), // todo
            animation: Animation::Standing,
            tween: None,
            sprite: data.sprite,
            direction: Direction::South,
        }
    }
    // pub fn tween_position(&self, time: f64) -> Vec2 {
    //     self.tween.as_ref().map_or_else(|| self.position.as_f32(), |tween| {
    //         let t = (time - tween.start) / (tween.end - tween.start);
    //         let diff = (self.position - tween.position).as_f32();

    //         tween.position.as_f32() + diff * t as f32
    //     })
    // }
    // pub fn set_tween(&mut self, position: IVec2, start: f64, duration: f64) {
    //     self.tween = Some(PlayerTween {
    //         position,
    //         start,
    //         end: start + duration,
    //     });
    // }
    pub fn draw(&self, time: f64, assets: &Assets) {
        self.draw_text(assets, self.position);
        self.draw_sprite(assets, self.position, time);
    }
    pub fn draw_text(&self, assets: &Assets, position: Vec2) {
        const FONT_SIZE: u16 = 16;
        let measurements = measure_text(&self.name, Some(assets.font), FONT_SIZE, 1.0);
        
        // ? The text is drawn with the baseline being the supplied y
        let text_offset = (
            (SPRITE_SIZE - measurements.width) / 2.0,
            -3.0,
        ).into();

        let outlines = &[
            (1.0, 0.0).into(),
            (-1.0, 0.0).into(),
            (0.0, 1.0).into(),
            (0.0, -1.0).into(),
            (-1.0, -0.0).into(),
            (-1.0, 1.0).into(),
            (1.0, -1.0).into(),
            (1.0, 1.0).into(),
        ];

        let pos = position + text_offset;

        for outline in outlines {
            let pos = pos + *outline;
            draw_text_ex(&self.name, pos.x, pos.y, TextParams {
                font_size: FONT_SIZE,
                font: assets.font,
                color: Color::new(0.0, 0.0, 0.0, 0.5),
                ..Default::default()
            });
        }

        // draw_text_ex(&self.name, pos.x + 1.0, pos.y + 1.0, TextParams {
        //     font_size: FONT_SIZE as _,
        //     font: assets.font,
        //     color: BLACK,
        //     ..Default::default()
        // });

        draw_text_ex(&self.name, pos.x, pos.y, TextParams {
                font_size: FONT_SIZE,
                font: assets.font,
                color: WHITE,
                ..Default::default()
            },
        );

        //draw_text(&self.name, pos.x, pos.y, FONT_SIZE, color_u8!(255, 192, 203, 255));
    }
    fn draw_sprite(&self, assets: &Assets, position: Vec2, time: f64) {
        let offset = self.animation.get_animation_offset(time, self.direction);

        let sprite_x = (self.sprite as f32 % 4.0) * 3.0;
        let sprite_y = (self.sprite as f32 / 4.0).floor() * 4.0;

        let source = Rect::new(
            sprite_x * SPRITE_SIZE + offset.x,
            sprite_y * SPRITE_SIZE + offset.y,
            SPRITE_SIZE,
            SPRITE_SIZE
        );

        draw_texture_ex(
            assets.sprites,
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

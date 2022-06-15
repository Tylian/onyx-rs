use crate::prelude::*;
use crate::game::Assets;

#[derive(Copy, Clone)]
pub struct PlayerTween {
    position: IVec2,
    start: f64,
    end: f64,
}

pub struct Player {
    pub id: ClientId,
    pub name: String,
    pub position: IVec2,
    pub tween: Option<PlayerTween>,
    pub sprite: u32,
    pub direction: Direction,
}

impl Player {
    pub fn from_network(id: ClientId, data: PlayerData) -> Self {
        Self {
            id,
            name: data.name,
            position: data.position.into(),
            tween: None,
            sprite: data.sprite,
            direction: Direction::South,
        }
    }
    pub fn tween_position(&self, time: f64) -> Vec2 {
        self.tween.as_ref().map_or_else(|| self.position.as_f32(), |tween| {
            let t = (time - tween.start) / (tween.end - tween.start);
            let diff = (self.position - tween.position).as_f32();

            tween.position.as_f32() + diff * t as f32
        })
    }
    pub fn set_tween(&mut self, position: IVec2, start: f64, duration: f64) {
        self.tween = Some(PlayerTween {
            position,
            start,
            end: start + duration,
        });
    }
    pub fn update(&mut self, time: f64) {
        if let Some(tween) = &self.tween {
            if tween.end <= time {
                self.tween = None;
            }
        }
    }
    pub fn draw(&self, time: f64, assets: &Assets) {
        let position = self.tween_position(time) * 48.0;

        self.draw_text(assets, position);
        self.draw_sprite(assets, position, time);
    }
    pub fn draw_text(&self, assets: &Assets, position: Vec2) {
        const FONT_SIZE: f32 = 16.0;
        let measurements = measure_text(&self.name, Some(assets.font), FONT_SIZE as _, 1.0);
        
        // ? The text is drawn with the baseline being the supplied y
        let text_offset = (
            (48.0 - measurements.width) / 2.0,
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
                font_size: FONT_SIZE as _,
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
                font_size: FONT_SIZE as _,
                font: assets.font,
                color: WHITE,
                ..Default::default()
            },
        );

        //draw_text(&self.name, pos.x, pos.y, FONT_SIZE, color_u8!(255, 192, 203, 255));
    }
    fn draw_sprite(&self, assets: &Assets, position: Vec2, time: f64) {
        let offset_y = match self.direction {
            Direction::South => 0.0,
            Direction::West => 1.0,
            Direction::East => 2.0,
            Direction::North => 3.0,
        };

        let offset_x = if let Some(tween) = &self.tween {
            let t = (time - tween.start) / (tween.end - tween.start);
            if t < 0.5 {
                0.0
            } else {
                2.0
            }
        } else {
            1.0
        };

        let sprite_x = (self.sprite as f32 % 4.0) * 3.0;
        let sprite_y = (self.sprite as f32 / 4.0).floor() * 4.0;

        let source = Rect::new(
            (sprite_x + offset_x) * 48.0,
            (sprite_y + offset_y) * 48.0,
            48.0,
            48.0
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

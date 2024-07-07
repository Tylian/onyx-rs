use onyx::{
    network::{Direction, Entity, Player as NetworkPlayer, PlayerFlags}, ACCELERATION, FRICTION, RUN_SPEED, SPRITE_SIZE, TILE_SIZE
};

use onyx::math::units::world::*;

use ggez::{glam::*, graphics::{Canvas, Color, DrawParam, PxScale, Text, TextAlign, TextLayout}};

use crate::{utils::{ping_pong, OutlinedText}, AssetCache};

pub enum Animation {
    Standing,
    Moving {
        /// Start time of the animation
        start: f32,
        /// Movement speed in pixels per second.
        speed: f32,
    },
}

impl Animation {
    fn get_animation_offset(&self, time: f32, direction: Direction) -> Vec2 {
        let offset_y = match direction {
            Direction::South => 0.0,
            Direction::West => 1.0,
            Direction::East => 2.0,
            Direction::North => 3.0,
        };

        let offset_x = match self {
            Animation::Standing => 1.0,
            Animation::Moving { start, speed } => {
                let length = 2.0 * TILE_SIZE / speed;
                ping_pong(((time - start) / length) % 1.0, 3) as f32
            }
        };

        vec2(offset_x * SPRITE_SIZE, offset_y * SPRITE_SIZE)
    }
}

pub struct Interpolation {
    pub initial: Point2D,
    pub target: Point2D,
    pub start_time: f32,
    pub duration: f32,
}

pub struct Player {
    pub id: Entity,
    pub name: String,
    pub position: Point2D,
    pub velocity: Vector2D,
    pub acceleration: Vector2D,
    pub test_acceleration: f32,
    pub test_friction: f32,
    pub max_speed: f32, // todo: eulcid::Scale?
    pub interpolation: Option<Interpolation>,
    pub last_update: f32,
    pub animation: Animation,
    pub sprite: u32,
    pub direction: Direction,
    pub flags: PlayerFlags,
}

impl Player {
    pub fn from_network(id: Entity, data: NetworkPlayer, time: f32) -> Self {
        let velocity = Vector2D::from(data.velocity);
        Self {
            id,
            name: data.name,
            position: data.position,
            interpolation: None,
            animation: if velocity == Vector2D::zero() {
                Animation::Moving {
                    start: time,
                    speed: velocity.length(),
                }
            } else {
                Animation::Standing
            },
            velocity,
            acceleration: Vector2D::zero(),
            test_acceleration: ACCELERATION,
            test_friction: FRICTION,
            max_speed: RUN_SPEED,
            sprite: data.sprite,
            direction: data.direction,
            last_update: time,
            flags: data.flags,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, time: f32, assets: &mut AssetCache) {
        self.draw_text(canvas, assets);
        self.draw_sprite(canvas, assets, time);
    }

    pub fn update_animation(&mut self, time: f32) {
        if self.velocity == Vector2D::zero() {
            self.animation = Animation::Standing;
        } else {
            let speed = self.velocity.length();
            self.animation = match self.animation {
                Animation::Standing => Animation::Moving {
                    start: time,
                    speed
                },
                Animation::Moving { start, .. } => Animation::Moving {
                    start, speed
                },
            }
        }
    }

    pub fn draw_text(&self, canvas: &mut Canvas, _assets: &mut AssetCache) {
        // let Some(font) = assets.font.lock() else {
        //     return;
        // };

        let text_offset = Vector2D::new(SPRITE_SIZE / 2.0, -3.0);
        let position = self.position.round() + text_offset;

        // todo: store in player?
        let mut text = Text::new(&self.name);
        text.set_layout(TextLayout {
            h_align: TextAlign::Middle,
            v_align: TextAlign::End,
        });
        text.set_scale(PxScale::from(16.0));

        canvas.draw(
            &OutlinedText::new(&text),
            DrawParam::default()
                .dest(position)
                .color(Color::WHITE)
        );

        // let mut text = ;
        // let params = DrawParam::default()
        //     .color(Color::WHITE)
        //     .dest(position);
        // canvas.draw(&text, params);
        

        // // const FONT_SIZE: u16 = 16;
        // // let measurements = measure_text(&self.name, Some(assets.font), FONT_SIZE, 1.0);

        // // // ? The text is drawn with the baseline being the supplied y
        // let text_offset = vec2(SPRITE_SIZE as f32 / 2.0, -3.0);
        // let position = self.position + text_offset;

        // draw_text_outline(
        //     draw,
        //     &font,
        //     &self.name,
        //     position,
        //     |text| {
        //         text.color(Color::WHITE)
        //             .v_align_bottom()
        //             .h_align_center()
        //             .size(16.0);
        //     }
        // )

        // // let pos = position + text_offset;
        // // draw_text_outline(
        // //     &self.name,
        // //     pos,
        // //     TextParams {
        // //         font_size: FONT_SIZE,
        // //         font: assets.font,
        // //         color: WHITE,
        // //         ..Default::default()
        // //     },
        // // );
    }

    fn draw_sprite(&self, canvas: &mut Canvas, assets: &mut AssetCache, time: f32) {
        use ggez::graphics::*;

        let offset = self.animation.get_animation_offset(time, self.direction);

        let sprite_x = (self.sprite as f32 % 4.0) * 3.0;
        let sprite_y = (self.sprite as f32 / 4.0).floor() * 4.0;

        let xy = vec2(
            sprite_x * SPRITE_SIZE + offset.x,
            sprite_y * SPRITE_SIZE + offset.y
        );

        let size = Vec2::splat(SPRITE_SIZE);

        let uv = assets.sprites.uv_rect(
            xy.x as u32,
            xy.y as u32,
            size.x as u32,
            size.y as u32
        );

        canvas.draw(&assets.sprites, DrawParam::default()
            .dest(self.position.round())
            .src(uv)
        );
    }

    // only block on the bottom half of the sprite, feels better
    pub fn collision_box(position: Point2D) -> Box2D {
        Box2D::from_origin_and_size(
            position + Vector2D::new(0.0, SPRITE_SIZE / 2.0),
            Size2D::new(SPRITE_SIZE, SPRITE_SIZE / 2.0)
        )
    }
}

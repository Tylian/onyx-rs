use euclid::approxeq::ApproxEq;
use ggez::glam::*;
use ggez::graphics::{Canvas, Color, DrawParam, PxScale, Text, TextAlign, TextLayout};
use onyx::math::units::world::*;
use onyx::network::{Direction, Entity, Interpolation, MapId, Player as NetworkPlayer, PlayerFlags, State};
use onyx::{RUN_SPEED, SPRITE_SIZE, TILE_SIZE};

use crate::utils::{ping_pong, OutlinedText};
use crate::AssetCache;

const FRAMES_PER_TILE: f32 = 2.0;
const FRAMES_PER_ANIMATION: u32 = 3;

#[derive(Debug, Clone, Copy)]
pub enum Animation {
    Standing {
        /// The current direction the animation is facing
        direction: Direction,
    },
    Moving {
        /// The time the animation was last progressed
        last_update: f32,
        /// The progress in the animation, in tiles
        progress: f32,
        /// The current direction the animation is facing
        direction: Direction
    },
}

impl Animation {
    /// Convenience function to get the current direction
    fn direction(&self) -> Direction {
        match self {
            Self::Standing { direction, ..} | Self::Moving { direction, .. } => *direction
        }
    }

    /// Create a new animation based on a velocity and start time
    pub fn new(velocity: Vector2D, start_time: f32) -> Self {
        let direction = Direction::from_velocity(velocity).unwrap_or(Direction::South);

        if velocity.approx_eq(&Vector2D::zero()) {
            Animation::Standing { direction }
        } else {
            Animation::Moving {
                last_update: start_time,
                progress: 0.0,
                direction,
            }
        }
    }

    /// Update an animation based on a movement velocity and the current time
    pub fn update(&mut self, velocity: Vector2D, time: f32) {
        let direction = Direction::from_velocity(velocity).unwrap_or(self.direction());

        if velocity.approx_eq(&Vector2D::zero()) {
            *self = Animation::Standing { direction };
        } else {
            let speed = velocity.length() / TILE_SIZE;
            *self = match self {
                Animation::Standing { .. } => Animation::Moving {
                    last_update: time,
                    progress: 0.0,
                    direction,
                },
                Animation::Moving { last_update, progress, .. } => Animation::Moving { 
                    last_update: time,
                    progress: *progress + (time - *last_update) * speed,
                    direction,
                },
            }
        }
    }

    /// Get the correct offset into the spritesheet for the current animation state
    pub fn get_animation_offset(&self) -> Vec2 {
        let offset_y = match self.direction() {
            Direction::South => 0.0,
            Direction::West => 1.0,
            Direction::East => 2.0,
            Direction::North => 3.0,
        };

        let offset_x = match self {
            Self::Standing { .. } => 1.0,
            Self::Moving { progress, .. } => {
                ping_pong((progress / FRAMES_PER_TILE).fract(), FRAMES_PER_ANIMATION) as f32
            }
        };

        vec2(offset_x * SPRITE_SIZE, offset_y * SPRITE_SIZE)
    }
}

#[derive(Debug)]
pub struct Player {
    pub id: Entity,
    pub name: String,

    pub position: Point2D,
    pub velocity: Vector2D,
    pub direction: Direction,
    pub map: MapId,
    pub max_speed: f32, // todo: eulcid::Scale?

    pub interpolation: Option<Interpolation>,
    pub animation: Animation,
    pub sprite: u32,

    pub flags: PlayerFlags,
    pub last_sequence_id: u64,
}

impl Player {
    pub fn from_network(id: Entity, data: NetworkPlayer, time: f32) -> Self {
        Self {
            id,
            map: data.map,
            name: data.name,
            position: data.position,
            interpolation: None,
            animation: Animation::new(data.velocity, time),
            velocity: data.velocity,
            max_speed: RUN_SPEED,
            sprite: data.sprite,
            direction: data.direction,
            flags: data.flags,
            last_sequence_id: 0,
        }
    }

    pub fn update_state(&mut self, state: State) {
        self.position = state.position;
        self.velocity = state.velocity;
        self.max_speed = state.max_speed;
        self.last_sequence_id = state.last_sequence_id;
        self.direction = state.direction;
        self.map = state.map;

        self.direction = Direction::from_velocity(state.velocity).unwrap_or(self.direction);
    }

    pub fn state(&self) -> State {
        State {
            id: self.id,
            position: self.position,
            velocity: self.velocity,
            direction: self.direction,
            map: self.map,
            max_speed: self.max_speed,
            last_sequence_id: self.last_sequence_id,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut AssetCache) {
        self.draw_text(canvas, assets);
        self.draw_sprite(canvas, assets);
    }

    pub fn update(&mut self, time: f32) {
        self.animation.update(self.velocity, time);
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
            DrawParam::default().dest(position).color(Color::WHITE),
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

    fn draw_sprite(&self, canvas: &mut Canvas, assets: &mut AssetCache) {
        use ggez::graphics::*;

        let offset = self.animation.get_animation_offset();

        let sprite_x = (self.sprite as f32 % 4.0) * 3.0;
        let sprite_y = (self.sprite as f32 / 4.0).floor() * 4.0;

        let xy = vec2(sprite_x * SPRITE_SIZE + offset.x, sprite_y * SPRITE_SIZE + offset.y);

        let size = Vec2::splat(SPRITE_SIZE);

        let uv = assets
            .sprites
            .uv_rect(xy.x as u32, xy.y as u32, size.x as u32, size.y as u32);

        canvas.draw(
            &assets.sprites,
            DrawParam::default().dest(self.position.round()).src(uv),
        );
    }

    // only block on the bottom half of the sprite, feels better
    pub fn collision_box(position: Point2D) -> Box2D {
        Box2D::from_origin_and_size(
            position + Vector2D::new(0.0, SPRITE_SIZE / 2.0),
            Size2D::new(SPRITE_SIZE, SPRITE_SIZE / 2.0),
        )
    }
}

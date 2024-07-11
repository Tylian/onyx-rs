pub mod math;
pub mod network;

pub const TILE_SIZE: f32 = 48.0;
pub const SPRITE_SIZE: f32 = 48.0;

pub const WALK_SPEED: f32 = 2.5 * TILE_SIZE;
pub const RUN_SPEED: f32 = 5.0 * TILE_SIZE;

pub const ACCELERATION: f32 = RUN_SPEED * 10.0;
pub const FRICTION: f32 = ACCELERATION * 0.3;

pub const SERVER_TICK_RATE: f32 = 30.0;
pub const SERVER_DELAY: f32 = 1.0 / SERVER_TICK_RATE;
pub const LERP_DURATION: f32 = 1.0 / SERVER_TICK_RATE;

// pub struct ScreenUnit;
// pub type ScreenPoint = Point2D<f32, ScreenUnit>;
// pub type ScreenVec = Vector2D<f32, ScreenUnit>;
// pub type ScreenBox2D = Box2D<f32, ScreenUnit>;
// pub type ScreenSize2D = Size2D<f32, ScreenUnit>;

// pub struct WorldUnit;
// pub type WorldPoint = Point2D<f32, WorldUnit>;
// pub type WorldVec = Vector2D<f32, WorldUnit>;
// pub type WorldBox2D = Box2D<f32, WorldUnit>;
// pub type WorldSize2D = Size2D<f32, WorldUnit>;

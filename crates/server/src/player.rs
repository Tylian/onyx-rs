use common::network::{Direction, Player as NetworkPlayer, PlayerFlags};
use euclid::default::{Point2D, Vector2D};

#[derive(Clone)]
pub struct Player {
    pub username: String,
    pub password: String,
    pub name: String,
    pub sprite: u32,
    pub position: Point2D<f32>,
    pub direction: Direction,
    pub velocity: Vector2D<f32>,
    pub map: String,
    pub flags: PlayerFlags,
}

impl From<Player> for NetworkPlayer {
    fn from(other: Player) -> Self {
        Self {
            name: other.name,
            sprite: other.sprite,
            velocity: other.velocity.into(),
            position: other.position.into(),
            direction: other.direction,
            flags: other.flags,
        }
    }
}

impl Player {}

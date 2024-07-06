use onyx::network::{Direction, Player as NetworkPlayer, PlayerFlags};
use onyx::math::units::world::*;

#[derive(Clone)]
pub struct Player {
    pub username: String,
    pub password: String,
    pub name: String,
    pub sprite: u32,
    pub position: Point2D,
    pub direction: Direction,
    pub velocity: Vector2D,
    pub map: String,
    pub flags: PlayerFlags,
}

impl From<Player> for NetworkPlayer {
    fn from(other: Player) -> Self {
        Self {
            name: other.name,
            sprite: other.sprite,
            velocity: other.velocity,
            position: other.position,
            direction: other.direction,
            flags: other.flags,
        }
    }
}

impl Player {}

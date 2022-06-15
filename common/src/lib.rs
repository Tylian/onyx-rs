// Notes on data:
pub mod network {
    use std::vec;

    use serde::{Serialize, Deserialize};
    use glam::IVec2;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone, Copy)]
    pub struct Coord(pub i32, pub i32);

    impl From<IVec2> for Coord {
        fn from(vec: IVec2) -> Self {
            Self(vec.x, vec.y)
        }
    }

    impl From<Coord> for IVec2 {
        fn from(coord: Coord) -> Self {
            IVec2::new(coord.0, coord.1)
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone, Copy)]
    pub struct Offset(pub i32, pub i32);

    impl From<IVec2> for Offset {
        fn from(vec: IVec2) -> Self {
            Self(vec.x, vec.y)
        }
    }

    impl From<Offset> for IVec2 {
        fn from(offset: Offset) -> Self {
            IVec2::new(offset.0, offset.1)
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, Copy)]
    #[serde(transparent)]
    pub struct ClientId(pub u64);

    impl From<u64> for ClientId {
        fn from(id: u64) -> Self {
            Self(id)
        }
    }
    
    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum ClientMessage {
        Move(Direction),
        Hello(String, u32),
        Message(String),
        ChangeTile(Coord, MapLayer, Option<Coord>, bool)
    }
    
    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum ServerMessage {
        Hello(ClientId),
        PlayerJoined(ClientId, PlayerData),
        PlayerLeft(ClientId),
        PlayerMoved(ClientId, Coord, Direction),
        Message(ChatChannel, String),
        ChangeTile(Coord, MapLayer, Option<Coord>, bool)
    }

    #[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum Direction {
        South,
        West,
        East,
        North,
    }

    impl Direction {
        pub fn reverse(&self) -> Self {
            match self {
                Self::South => Self::North,
                Self::West => Self::East,
                Self::East => Self::West,
                Self::North => Self::North,
            }
        }
        pub fn offset(&self) -> Offset {
            match self {
                Direction::South => Offset(0, 1),
                Direction::West => Offset(-1, 0),
                Direction::East => Offset(1, 0),
                Direction::North => Offset(0, -1),
            }
        }
    }

    impl From<Direction> for Offset {
        fn from(dir: Direction) -> Self {
           dir.offset()
        }
    }

    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub struct PlayerData {
        pub name: String,
        pub position: Coord,
        pub sprite: u32,
    }

    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum ChatChannel {
        Server,
        Say
    }
    
    #[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum MapLayer {
        Ground,
        Mask,
        Fringe,
    }

    impl MapLayer {
        pub fn iter() -> impl Iterator<Item = MapLayer> {
            vec![MapLayer::Ground, MapLayer::Mask, MapLayer::Fringe].into_iter()
        }
    }
}
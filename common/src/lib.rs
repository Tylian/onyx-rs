pub mod network {
    use serde::{Serialize, Deserialize};

    type Vec2 = (f32, f32);

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
        Move(Vec2, Direction),
        Hello(String, u32),
        Message(String),
        ChangeTile(Vec2, u32)
    }
    
    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum ServerMessage {
        Hello(ClientId),
        PlayerJoined(ClientId, PlayerData),
        PlayerLeft(ClientId),
        PlayerMoved(ClientId, Vec2, Vec2, Direction),
        Message(ChatChannel, String),
        ChangeTile(Vec2, u32)
    }

    #[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum Direction {
        South,
        West,
        East,
        North,
    }

    impl From<Direction> for glam::Vec2 {
        fn from(dir: Direction) -> Self {
            match dir {
                Direction::South => glam::vec2(0.0, 1.0),
                Direction::West => glam::vec2(-1.0, 0.0),
                Direction::East => glam::vec2(1.0, 0.0),
                Direction::North => glam::vec2(0.0, -1.0),
            }
        }
    }

    impl From<Direction> for (f32, f32) {
        fn from(dir: Direction) -> Self {
            match dir {
                Direction::South => (0.0, 1.0),
                Direction::West => (-1.0, 0.0),
                Direction::East => (1.0, 0.0),
                Direction::North => (0.0, -1.0),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub struct PlayerData {
        pub name: String,
        pub position: Vec2,
        pub sprite: u32,
    }

    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    pub enum ChatChannel {
        Server,
        Say
    }
}
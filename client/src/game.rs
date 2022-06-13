
use common::network::{ClientId, ChatChannel, ServerMessage, ClientMessage, Direction};

use crate::{prelude::*, networking::{Networking, NetworkStatus}, player::Player, map::Map};

const MOVEMENT_SPEED: f64 = 1.0 / 5.0;

pub struct GameState {
    chat_message: String,
    chat_messages: Vec<(ChatChannel, String)>,
    movement_lock: f64,
    my_id: Option<ClientId>,
    players: Vec<Player>,
    network: Networking,
    map: Map
}

impl GameState {
    fn from_network(network: Networking) -> Self {
        Self {
            chat_message: Default::default(),
            chat_messages: Default::default(),
            movement_lock: Default::default(),
            my_id: Default::default(),
            players: Default::default(),
            network,
            map: Map::new(20, 15)
        }
    }

    fn me(&self) -> Option<&Player> {
        self.my_id.and_then(|my_id| self.players.iter().find(|p| p.id == my_id))
    }

    fn me_mut(&mut self) -> Option<&mut Player> {
        self.my_id.and_then(|my_id| self.players.iter_mut().find(|p| p.id == my_id))
    }


    fn update_network(&mut self, time: f64) {
        if self.network.status() != NetworkStatus::Connected {
            return;
        }

        while let Some(message) = self.network.try_recv() {
            println!("{:?}", message);
            match message {
                ServerMessage::Hello(client_id) => {
                    self.my_id = Some(client_id);
                }
                ServerMessage::PlayerJoined(client_id, player_data) => {
                    self.players
                        .push(Player::from_network(client_id, player_data));
                }
                ServerMessage::PlayerLeft(client_id) => {
                    // self.players.retain(|p| p.id != client_id);
                    let idx = self.players.iter().position(|p| p.id == client_id);
                    if let Some(idx) = idx {
                        self.players.swap_remove(idx);
                    }
                }
                ServerMessage::PlayerMoved(client_id, from, to, direction) => {
                    let mut player = self.players.iter_mut().find(|p| p.id == client_id).unwrap();
                    player.position = to.into();
                    player.direction = direction;
                    player.set_tween(from.into(), time, MOVEMENT_SPEED);
                }
                ServerMessage::Message(channel, message) => {
                    self.chat_messages.push((channel, message))
                },
                ServerMessage::ChangeTile((x, y), index) => {
                    let tile = self.map.tile_mut(x as u32, y as u32).unwrap();
                    tile.index = index;
                },
                
            }
        }
    }
    fn update_ui(&mut self, ctx: &egui::Context) {
        use egui::{*, style::Margin};

        let chat_window = Window::new("💬 Chat")
            .resizable(true)
            .default_pos([7.0, 522.0])
            .default_size([367.0, 147.0])
            .min_height(125.0);

        // 7 522 386 708
        chat_window.show(&ctx, |ui| {
            TopBottomPanel::bottom("chat_bottom")
                .frame(Frame::none().inner_margin(Margin::symmetric(8.0, 2.0)))
                .show_inside(ui, |ui| {
                    ui.separator();
                    ui.with_layout(Layout::right_to_left(), |ui| {
                        let button = ui.button("Send");
                        let text = ui.add_sized(
                            ui.available_size(),
                            TextEdit::singleline(&mut self.chat_message),
                        );

                        if (text.lost_focus() && ui.input().key_pressed(Key::Enter))
                            || button.clicked()
                        {
                            let text = std::mem::take(&mut self.chat_message);
                            self.network.send(ClientMessage::Message(text));
                        }
                    });
                });

            CentralPanel::default().show_inside(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom()
                    .show(ui, |ui| {
                        for (channel, message) in &self.chat_messages {
                            let color = match channel {
                                ChatChannel::Server => Color32::YELLOW,
                                ChatChannel::Say => Color32::WHITE,
                            };

                            let prefix = match channel {
                                ChatChannel::Server => "[Server] ",
                                ChatChannel::Say => "",
                            };
                            ui.colored_label(color, format!("{}{}", prefix, message));
                        }
                    });
            });
        });

        /*egui::Window::new("📝 Memory")
        .resizable(false)
        .show(&egui_ctx, |ui| {
            egui_ctx.memory_ui(ui);
        });*/
    }
}

pub struct Assets {
    pub tileset: Texture2D,
    pub sprites: Texture2D,
    pub font: Font,
}

impl Assets {
    async fn load() -> GameResult<Self> {
        Ok(Self {
            tileset: load_texture("assets/Outside_A2.png").await?,
            sprites: load_texture("assets/Actor1.png").await?,
            font: load_ttf_font("assets/LiberationMono-Regular.ttf").await?,
        })
    }
}

pub async fn game_screen(network: Networking) {
    let assets = Assets::load().await
        .expect("Could not load assets");

    let mut state = GameState::from_network(network);

    loop {
        let time = get_time();

        // update
        state.update_network(time);
        egui_macroquad::ui(|ctx| state.update_ui(ctx));
        
        for player in state.players.iter_mut() {
            player.update(time);
        }

        if state.my_id.is_some() {
            if state.movement_lock <= time {
                let movement = if is_key_down(KeyCode::Up) {
                    Some(Direction::North)
                } else if is_key_down(KeyCode::Down) {
                    Some(Direction::South)
                } else if is_key_down(KeyCode::Left) {
                    Some(Direction::West)
                } else if is_key_down(KeyCode::Right) {
                    Some(Direction::East)
                } else {
                    None
                };

                if let Some(direction) = movement {
                    // ! This is a hack due to Macroquad's Glam re-exporting without mint shit.
                    let vec: (f32, f32) = direction.into(); 
                    let new_position = state.me().unwrap().position + vec.into();

                    if state.map.valid(new_position) {
                        let me = state.me_mut().unwrap();
                        let from = me.position;
                        me.position = new_position;
                        me.direction = direction;

                        me.set_tween(from, time, MOVEMENT_SPEED);

                        state.network.send(ClientMessage::Move(new_position.into(), direction));
                        state.movement_lock = time + MOVEMENT_SPEED;
                    }
                }
            }
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let (tx, ty) = (mx as u32 / 48, my as u32 / 48);

            let tile = state.map.tile_mut(tx, ty).unwrap();
            tile.index += 1;

            state.network.send(ClientMessage::ChangeTile((tx as f32, ty as f32), tile.index));
        }

        // draw
        clear_background(BLACK);

        state.map.draw(&assets);

        for player in &state.players {
            player.draw(time, &assets);
        }

        egui_macroquad::draw();

        next_frame().await;
    }
}
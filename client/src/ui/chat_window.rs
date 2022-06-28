use common::network::ChatChannel;
use egui::{Color32, Key, Response, ScrollArea, Ui, Window};
use egui_extras::{Size, StripBuilder};
use macroquad::window::screen_height;

pub type ChatMessage = (ChatChannel, String);

pub struct ChatWindow {
    buffer: Vec<ChatMessage>,
    channel: ChatChannel,
    message: String,
    send_message: Option<ChatMessage>,
}

impl ChatWindow {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            channel: ChatChannel::Say,
            message: String::new(),
            send_message: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        Window::new("💬 Chat")
            .resizable(true)
            .default_pos(egui::pos2(7., screen_height() - 198.)) // idfk lmao
            .default_size([367., 147.])
            .min_height(125.)
            .show(ctx, |ui| self.ui(ui));
    }

    pub fn insert(&mut self, channel: ChatChannel, message: String) {
        self.buffer.push((channel, message));
    }

    pub fn message(&mut self) -> Option<ChatMessage> {
        self.send_message.take()
    }

    fn ui(&mut self, ui: &mut Ui) {
        let mut text: Option<Response> = None;
        let mut button: Option<Response> = None;

        let bottom_height = ui.spacing().interact_size.y;
        StripBuilder::new(ui)
            .size(Size::remainder().at_least(100.))
            .size(Size::exact(6.))
            .size(Size::exact(bottom_height))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .stick_to_bottom()
                        .show(ui, |ui| {
                            for (channel, message) in &self.buffer {
                                self.message_ui(ui, *channel, message);
                            }
                        });
                });
                strip.cell(|ui| {
                    ui.separator();
                });
                strip.strip(|builder| {
                    builder
                        .size(Size::exact(40.))
                        .size(Size::remainder())
                        .size(Size::exact(40.))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.colored_label(Color32::WHITE, "Say:");
                            });
                            strip.cell(|ui| {
                                text = Some(ui.text_edit_singleline(&mut self.message));
                            });
                            strip.cell(|ui| {
                                button = Some(ui.button("Send"));
                            });
                        });
                });
            });

        if let Some((text, button)) = text.zip(button) {
            if (text.lost_focus() && ui.input().key_pressed(Key::Enter)) || button.clicked() {
                let message = std::mem::take(&mut self.message);
                self.send_message = Some((self.channel, message));
                text.request_focus();
            }
        }
    }

    fn message_ui(&self, ui: &mut egui::Ui, channel: ChatChannel, message: &str) {
        match channel {
            ChatChannel::Echo => {
                ui.colored_label(Color32::WHITE, message);
            }
            ChatChannel::Server => {
                ui.colored_label(Color32::GOLD, format!("[Server] {}", message));
            }
            ChatChannel::Say => {
                ui.colored_label(Color32::WHITE, format!("[Say] {}", message));
            }
            ChatChannel::Global => {
                ui.colored_label(Color32::LIGHT_BLUE, format!("[Global] {}", message));
            }
        };
    }
}

use onyx::network::ChatChannel;
use ggegui::egui::{Align2, Color32, ComboBox, Context, Key, Response, RichText, ScrollArea, Ui, Window};
use egui_extras::{Size, StripBuilder};
// use macroquad::window::screen_height;

pub type ChatMessage = (ChatChannel, String);

pub struct ChatWindow {
    buffer: Vec<ChatMessage>,
    channel: ChatChannel,
    message: String,
    send_message: Option<ChatMessage>,
}

fn channel_info(channel: ChatChannel) -> (Color32, &'static str) {
    match channel {
        ChatChannel::Echo => (Color32::LIGHT_GRAY, "Echo"),
        ChatChannel::Server => (Color32::GOLD, "Server"),
        ChatChannel::Say => (Color32::WHITE, "Say"),
        ChatChannel::Global => (Color32::from_rgb(0x75, 0x6d, 0xd1), "Global"),
        ChatChannel::Error => (Color32::RED, "Error"),
    }
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

    pub fn show(&mut self, ctx: &Context, screen_height: f32) {
        Window::new("💬 Chat")
            // .resizable(true)
            .pivot(Align2::LEFT_BOTTOM)
            .fixed_pos((7.0, screen_height - 7.0)) // idfk lmao
            .fixed_size([367.0, 147.0])
            // .min_height(125.0)
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
            .size(Size::remainder().at_least(100.0))
            .size(Size::exact(6.0))
            .size(Size::exact(bottom_height))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
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
                        .size(Size::exact(65.0))
                        .size(Size::remainder())
                        .size(Size::exact(40.0))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                fn channel_label(channel: ChatChannel) -> RichText {
                                    let (color, text) = channel_info(channel);
                                    RichText::new(text).color(color)
                                }

                                ComboBox::from_id_source("chat channel")
                                    .selected_text(channel_label(self.channel))
                                    .width(65.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.channel,
                                            ChatChannel::Say,
                                            channel_label(ChatChannel::Say),
                                        );
                                        ui.selectable_value(
                                            &mut self.channel,
                                            ChatChannel::Global,
                                            channel_label(ChatChannel::Global),
                                        );
                                        ui.selectable_value(
                                            &mut self.channel,
                                            ChatChannel::Server,
                                            channel_label(ChatChannel::Server),
                                        );
                                    });
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
            if (text.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) || button.clicked() {
                let message = std::mem::take(&mut self.message);
                self.send_message = Some((self.channel, message));
                text.request_focus();
            }
        }
    }

    fn message_ui(&self, ui: &mut Ui, channel: ChatChannel, message: &str) {
        let (color, name) = channel_info(channel);
        match channel {
            ChatChannel::Echo | ChatChannel::Error => {
                ui.colored_label(color, message);
            }
            ChatChannel::Server | ChatChannel::Say | ChatChannel::Global => {
                ui.colored_label(color, format!("[{name}] {message}"));
            }
        };
    }
}

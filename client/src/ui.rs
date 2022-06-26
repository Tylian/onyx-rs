mod map_editor;

use egui::{TextureHandle, ScrollArea, Ui, Image, Sense, Rect, popup_below_widget, Response, Id};
pub use map_editor::*;

use common::SPRITE_SIZE;

use crate::utils::ping_pong;

// TODO multiple tile selections
pub fn tile_selector(ui: &mut Ui, texture: &TextureHandle, selected: &mut egui::Pos2, snap: egui::Vec2) {
    ScrollArea::both().show_viewport(ui, |ui, viewport| {
        let clip_rect = ui.clip_rect();

        let margin = ui.visuals().clip_rect_margin;
        let offset = (clip_rect.left_top() - viewport.left_top()) + egui::vec2(margin, margin);
        let texture_size = texture.size_vec2();

        let response = ui.add(Image::new(texture, texture_size).sense(Sense::click()));
        if response.clicked() {
            let pointer = response.interact_pointer_pos().unwrap();
            let position = pointer - offset;
            if position.x >= 0.0 && position.y >= 0.0 && position.x < texture_size.x && position.y < texture_size.y {
                *selected = (snap * (position.to_vec2() / snap).floor()).to_pos2();
            }
        }

        let painter = ui.painter();
        let rect = Rect::from_min_size(*selected + offset, snap);
        painter.rect_stroke(rect, 0., ui.visuals().window_stroke());

        response
    });
}

pub fn sprite_preview(ui: &mut Ui, texture: &TextureHandle, time: f64, sprite: u32) -> Response {
    let sprite_x = (sprite as f64 % 4.0) * 3.0;
    let sprite_y = (sprite as f64 / 4.0).floor() * 4.0;

    // walk left and right
    let speed = 2.5; // tiles per second
    let loops = 8.0; // how many tiles to walk before rotating

    let animation_speed = 2.0 / speed; // time to complete 1 walk cycle

    let offset_x = ping_pong(time / animation_speed % 1.0, 3) as f64;
    let offset_y = ((time / (animation_speed * loops)) % 4.0).floor();

    let p = egui::vec2(
        (sprite_x + offset_x) as f32 * SPRITE_SIZE as f32,
        (sprite_y + offset_y) as f32 * SPRITE_SIZE as f32,
    ) / texture.size_vec2();
    let size = egui::vec2(SPRITE_SIZE as f32, SPRITE_SIZE as f32) / texture.size_vec2();
    let sprite = Image::new(texture, (SPRITE_SIZE as f32, SPRITE_SIZE as f32))
        .uv(Rect::from_min_size(p.to_pos2(), size));

    ui.add(sprite)
}

fn auto_complete<T: AsRef<str>>(ui: &mut Ui, popup_id: Id, suggestions: &[T], current: &mut String) {
    let filtered = suggestions
        .iter()
        .filter(|item| item.as_ref().contains(&*current))
        .collect::<Vec<_>>();

    let text_edit = ui.text_edit_singleline(current);
    if text_edit.gained_focus() {
        ui.memory().open_popup(popup_id);
    }

    popup_below_widget(ui, popup_id, &text_edit, |ui| {
        ScrollArea::vertical()
            .max_height(ui.spacing().combo_height)
            .show(ui, |ui| {
                for item in filtered {
                    let item = item.as_ref();
                    if ui.selectable_label(current == item, item).clicked() {
                        *current = String::from(item);
                    }
                }
            })
            .inner
    });

    // crappy attempt at fixing a bug lmao
    if text_edit.lost_focus() {
        ui.memory().close_popup();
    }
}

fn option_combo<H, T, F>(ui: &mut Ui, id: H, selected: &mut Option<T>, render: F, list: &[T])
where
    H: std::hash::Hash,
    T: PartialEq + Clone,
    F: Fn(Option<&T>) -> String,
{
    egui::ComboBox::from_id_source(id)
        .selected_text(render(selected.as_ref()))
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, None, render(None));
            ui.separator();

            for item in list.iter() {
                if ui
                    .selectable_label(selected.as_ref() == Some(item), render(Some(item)))
                    .clicked()
                {
                    *selected = Some(item.clone())
                }
            }
        });
}

fn option_textedit(ui: &mut Ui, value: &mut Option<String>) -> Response {
    ui.horizontal(|ui| {
        let mut enabled = value.is_some();
        if ui.checkbox(&mut enabled, "").changed() && enabled != value.is_some() {
            if enabled {
                *value = Some(String::new());
            } else {
                *value = None;
            }
        }

        ui.add_enabled_ui(enabled, |ui| match value.as_mut() {
            Some(text) => ui.text_edit_singleline(text),
            None => ui.label("disabled"),
        }).inner
    })
    .inner
}

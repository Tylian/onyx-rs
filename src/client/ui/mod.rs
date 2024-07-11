#![allow(dead_code)]

mod chat_window;
mod map_editor;

use ggegui::egui::*;

use onyx::SPRITE_SIZE;
use load::SizedTexture;

use crate::utils::ping_pong;

pub use self::chat_window::*;
pub use self::map_editor::*;

// ! A few functions in here are dead code, remove them if need be eventually.
pub fn dialog<R>(ctx: &Context, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    // vaguely based on the following code, thanks!
    // https://github.com/4JX/mCubed/blob/8a3b0a1568cbca3c372416db9a82f69b088ae0c6/main/src/ui/widgets/screen_prompt.rs
    Area::new(Id::new("prompt_bg")).fixed_pos(Pos2::ZERO).show(ctx, |ui| {
        let screen_rect = ctx.input(|i| i.screen_rect);

        ui.allocate_response(screen_rect.size(), Sense::click());

        // 50% opacity non-interative color
        let shade_color = ui.visuals().noninteractive().bg_fill.linear_multiply(0.5);

        ui.painter()
            .add(Shape::rect_filled(screen_rect, Rounding::ZERO, shade_color));

        Area::new(Id::new("prompt_centered"))
            .anchor(Align2::CENTER_CENTER, Vec2::splat(0.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                Frame::popup(ui.style())
                    .show(ui, |ui| {
                        Resize::default()
                            .auto_sized()
                            .with_stroke(false)
                            .min_size([96.0, 32.0])
                            .default_size([340.0, 420.0])
                            .show(ui, add_contents)
                    })
                    .inner
            })
            .inner
    })
}

// TODO multiple tile selections
pub fn tile_selector(ui: &mut Ui, texture: SizedTexture, selected: &mut Pos2, snap: Vec2) {
    ScrollArea::both().show_viewport(ui, |ui, viewport| {
        let clip_rect = ui.clip_rect();

        let margin = ui.visuals().clip_rect_margin;
        let offset = (clip_rect.left_top() - viewport.left_top()) + vec2(margin, margin);

        let response = ui.add(Image::new(texture).sense(Sense::click()));
        if response.clicked() {
            let pointer = response.interact_pointer_pos().unwrap();
            let position = pointer - offset;
            if position.x >= 0.0 && position.y >= 0.0 && position.x < texture.size.x && position.y < texture.size.y {
                *selected = (snap * (position.to_vec2() / snap).floor()).to_pos2();
            }
        }

        let painter = ui.painter();
        let rect = Rect::from_min_size(*selected + offset, snap);
        painter.rect_stroke(rect, 0.0, ui.visuals().window_stroke());

        response
    });
}
#[allow(dead_code)] // keeping it for a rainy day
pub fn sprite_preview(ui: &mut Ui, texture: &TextureHandle, time: f32, sprite: u32) -> Response {
    let sprite_x = (sprite as f32 % 4.0) * 3.0;
    let sprite_y = (sprite as f32 / 4.0).floor() * 4.0;

    // walk left and right
    let speed = 2.5; // tiles per second
    let loops = 8.0; // how many tiles to walk before rotating

    let animation_speed = 2.0 / speed; // time to complete 1 walk cycle

    let offset_x = ping_pong(time / animation_speed % 1.0, 3) as f32;
    let offset_y = ((time / (animation_speed * loops)) % 4.0).floor();

    let p = vec2(
        (sprite_x + offset_x) * SPRITE_SIZE,
        (sprite_y + offset_y) * SPRITE_SIZE,
    ) / texture.size_vec2();
    let size = vec2(SPRITE_SIZE, SPRITE_SIZE) / texture.size_vec2();
    let sprite = Image::from_texture(texture)
        .fit_to_exact_size(vec2(SPRITE_SIZE, SPRITE_SIZE))
        .uv(Rect::from_min_size(p.to_pos2(), size));

    ui.add(sprite)
}

#[allow(dead_code)] // keeping it for a rainy day
fn auto_complete<T: AsRef<str>>(ui: &mut Ui, popup_id: Id, suggestions: &[T], current: &mut String) {
    let filtered = suggestions
        .iter()
        .filter(|item| item.as_ref().contains(&*current))
        .collect::<Vec<_>>();

    let text_edit = ui.text_edit_singleline(current);
    if text_edit.gained_focus() {
        ui.memory_mut(|m| m.open_popup(popup_id));
    }

    popup_below_widget(ui, popup_id, &text_edit, PopupCloseBehavior::CloseOnClickOutside, |ui| {
        ScrollArea::vertical()
            .max_height(ui.spacing().combo_height)
            .show(ui, |ui| {
                for item in filtered {
                    let item = item.as_ref();
                    if ui.selectable_label(current == item, item).clicked() {
                        *current = String::from(item);
                    }
                }
            });
    });

    // crappy attempt at fixing a bug lmao
    if text_edit.lost_focus() {
        ui.memory_mut(|m| m.close_popup());
    }
}

#[allow(dead_code)] // keeping it for a rainy day
fn option_combo<H, T, F>(ui: &mut Ui, id: H, selected: &mut Option<T>, render: F, list: &[T])
where
    H: std::hash::Hash,
    T: PartialEq + Clone,
    F: Fn(Option<&T>) -> String,
{
    ComboBox::from_id_source(id)
        .selected_text(render(selected.as_ref()))
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, None, render(None));
            ui.separator();

            for item in list.iter() {
                if ui
                    .selectable_label(selected.as_ref() == Some(item), render(Some(item)))
                    .clicked()
                {
                    *selected = Some(item.clone());
                }
            }
        });
}

#[allow(dead_code)] // keeping it for a rainy day
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
        })
        .inner
    })
    .inner
}

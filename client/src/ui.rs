use egui::{Ui, Response, Pos2, Sense, Image, Vec2, ScrollArea, TextureHandle, Rect};

pub fn attribute_radio(ui: &mut Ui, selected: bool, title: &str, description: &str) -> Response {
    ui.radio(selected, title).on_hover_ui(|ui| {
        ui.heading(title);
        ui.label(description);
    })
}

pub fn tile_selector(ui: &mut Ui, texture: &TextureHandle, selected: &mut Pos2, snap: Vec2) {
    ScrollArea::both().show_viewport(ui, |ui, viewport| {
        let clip_rect = ui.clip_rect();
        let response = ui.add(Image::new(texture, texture.size_vec2()).sense(Sense::click()));
        if response.clicked() {
            let pos = response.interact_pointer_pos().expect("Pointer position shouldn't be None");
            let offset = viewport.left_top() + (pos - clip_rect.left_top()); // weird order just to make it typecheck lol
            *selected = (snap * (offset.to_vec2() / snap).floor()).to_pos2();
        }
        let rect = Rect::from_min_size(*selected, snap);

        // todo: this is offset slightly by the stroke?
        let painter = ui.painter();
        painter.rect_stroke(rect, 0., ui.visuals().window_stroke());

        response
    });
}
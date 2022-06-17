use macroquad::prelude::*;
use crate::GameResult;

#[derive(Clone)]
pub struct Assets {
    pub tileset: Texture2D,
    pub sprites: Texture2D,
    pub font: Font,
    pub egui: EguiAssets
}

#[derive(Default, Clone)]
pub struct EguiAssets {
    pub tileset: Option<egui::TextureHandle>,
    pub sprites: Option<egui::TextureHandle>,
}

impl Assets {
    pub async fn load() -> GameResult<Self> {
        Ok(Self {
            tileset: load_texture("assets/tileset1.png").await?,
            sprites: load_texture("assets/sprites.png").await?,
            font: load_ttf_font("assets/LiberationMono-Regular.ttf").await?,
            egui: Default::default()
        })
    }

    pub fn load_egui(&mut self, ctx: &egui::Context) {
        self.egui.sprites.get_or_insert_with(|| Self::load_egui_texture(ctx, "sprites", self.sprites));
        self.egui.tileset.get_or_insert_with(|| Self::load_egui_texture(ctx, "tileset", self.tileset));
    }

    fn load_egui_texture(ctx: &egui::Context, name: impl ToString, texture: Texture2D) -> egui::TextureHandle {
        let image = texture.get_texture_data();
        let size = [image.width(), image.height()];
        let egui_image = egui::ColorImage::from_rgba_unmultiplied(size, &image.bytes);
        ctx.load_texture(name.to_string(), egui_image)
    }
}
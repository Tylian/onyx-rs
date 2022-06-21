use crate::GameResult;
use macroquad::prelude::*;

#[derive(Clone)]
pub struct Assets {
    pub tileset: Texture2D,
    pub sprites: Texture2D,
    pub font: Font,
    pub egui: EguiTextures,
}

#[derive(Default, Clone)]
pub struct EguiTextures {
    pub tileset: Option<egui::TextureHandle>,
    pub sprites: Option<egui::TextureHandle>,
}

impl Assets {
    pub async fn load() -> GameResult<Self> {
        let tileset = load_texture("assets/tileset1.png").await?;
        let sprites = load_texture("assets/sprites.png").await?;

        tileset.set_filter(FilterMode::Nearest);
        sprites.set_filter(FilterMode::Nearest);

        Ok(Self {
            tileset,
            sprites,
            font: load_ttf_font("assets/LiberationMono-Regular.ttf").await?,
            egui: EguiTextures::default(),
        })
    }

    pub fn load_egui(&mut self, ctx: &egui::Context) {
        self.egui.sprites.get_or_insert_with(|| {
            Self::load_egui_texture(ctx, "sprites", self.sprites)
        });
        self.egui.tileset.get_or_insert_with(|| {
            Self::load_egui_texture(ctx, "tileset", self.tileset)
        });
    }

    fn load_egui_texture(
        ctx: &egui::Context,
        name: &str,
        texture: Texture2D,
    ) -> egui::TextureHandle {
        use egui::ColorImage;

        let image = texture.get_texture_data();
        let size = [image.width(), image.height()];
        let image = ColorImage::from_rgba_unmultiplied(size, &image.bytes);
        ctx.load_texture(name, image)
    }
}

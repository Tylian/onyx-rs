use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    ffi::OsStr,
};

use anyhow::{anyhow, Result};
use macroquad::prelude::*;

#[derive(Clone)]
pub struct DualTexture {
    pub name: String,
    pub texture: Texture2D,
    pub egui: egui::TextureHandle,
}

impl DualTexture {
    fn from_image(name: &str, image: &Image) -> Self {
        let texture = Texture2D::from_image(image);
        texture.set_filter(FilterMode::Nearest);

        let mut egui: Option<egui::TextureHandle> = None;
        egui_macroquad::cfg(|ctx| {
            let size = [image.width(), image.height()];
            let image = egui::ColorImage::from_rgba_unmultiplied(size, &image.bytes);
            egui = Some(ctx.load_texture(name, image));
        });

        Self {
            name: name.to_string(),
            texture,
            egui: egui.expect("Could not convert texture to egui, impossible??"),
        }
    }
}

#[derive(Clone)]
pub struct Assets {
    tilesets: HashMap<String, Image>,
    pub tileset: RefCell<DualTexture>,
    pub sprites: DualTexture,
    pub font: Font,
}

impl Assets {
    pub async fn load() -> Result<Self> {
        let sprites = load_image("assets/sprites.png").await?;
        let sprites = DualTexture::from_image("sprites.png", &sprites);
        let font = load_ttf_font("assets/LiberationMono-Regular.ttf").await?;

        let tilesets = Assets::load_tilesets().await?;
        // unwrap: Assets::load_tilesets ensures that at least "default.png" always exists
        let tileset = DualTexture::from_image("default.png", tilesets.get("default.png").unwrap());

        Ok(Self {
            tilesets,
            tileset: RefCell::new(tileset),
            sprites,
            font,
        })
    }

    async fn load_tilesets() -> Result<HashMap<String, Image>> {
        let mut tilesets = HashMap::new();
        for entry in std::fs::read_dir("./assets/tilesets")? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("png") {
                debug!("Loading tileset {}", path.display());
                let image = load_image(&path.to_string_lossy()).await?;
                let name = path.file_name().unwrap().to_string_lossy();
                tilesets.insert(name.to_string(), image);
            }
        }

        if !tilesets.contains_key("default.png") {
            Err(anyhow!(
                "the file \"./assets/tilesets/default.png\" does not exist, but it is required to exist"
            ))
        } else {
            Ok(tilesets)
        }
    }

    pub fn tileset(&self) -> Ref<'_, DualTexture> {
        self.tileset.borrow()
    }

    pub fn tilesets(&self) -> Vec<&str> {
        self.tilesets.keys().map(|x| &**x).collect()
    }

    pub fn set_tileset(&self, name: &str) -> Result<()> {
        if let Some(image) = self.tilesets.get(name) {
            if self.tileset.borrow().name != name {
                self.tileset.replace(DualTexture::from_image(name, image));
            }
            Ok(())
        } else {
            Err(anyhow!("not found"))
        }
    }
}

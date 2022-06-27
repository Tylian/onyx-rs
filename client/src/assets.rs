use std::{cell::{Ref, RefCell}, collections::HashMap, ffi::OsStr, fs::File, io::BufReader, path::{PathBuf, Path}};

use anyhow::{anyhow, Result};
use macroquad::prelude::*;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

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

pub struct Assets {
    tilesets: HashMap<String, Image>,
    pub tileset: RefCell<DualTexture>,
    pub sprites: DualTexture,
    pub font: Font,

    _output_stream: OutputStream,
    stream_handle: OutputStreamHandle,

    music_list: Vec<String>,
    current_sink: RefCell<Option<(String, Sink)>>,
}

impl Assets {
    /// Convenience function that returns an asset path in the runtime folder
    fn asset_path(source: impl AsRef<Path>) -> PathBuf {
        let mut path = common::client_runtime!();
        path.push("assets");
        path.push(source); 
        path
    }

    /// Convenience function that returns an asset path as a string
    fn asset_path_str(source: impl AsRef<Path>) -> String {
        Self::asset_path(source).to_string_lossy().to_string()
    }

    pub async fn load() -> Result<Self> {
        let sprites = load_image(&Self::asset_path_str("sprites.png")).await?;
        let sprites = DualTexture::from_image("sprites.png", &sprites);
        let font = load_ttf_font(&Self::asset_path_str("LiberationMono-Regular.ttf")).await?;

        let tilesets = Assets::load_tilesets().await?;
        let music_list = Assets::load_music_list().await?;

        // unwrap: Assets::load_tilesets ensures that at least "default.png" always exists
        let tileset = DualTexture::from_image("default.png", tilesets.get("default.png").unwrap());
        let (stream, stream_handle) = OutputStream::try_default()?;

        Ok(Self {
            tilesets,
            tileset: RefCell::new(tileset),
            music_list,
            current_sink: RefCell::new(None),
            sprites,
            font,
            _output_stream: stream,
            stream_handle,
        })
    }

    async fn load_tilesets() -> Result<HashMap<String, Image>> {
        let mut tilesets = HashMap::new();
        for entry in std::fs::read_dir(Self::asset_path("tilesets"))? {
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
                "the file \"{}\" does not exist, but it is required to exist"
            , Self::asset_path("tilesets/default.png").display()))
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

    pub fn get_music(&self) -> Vec<String> {
        self.music_list.clone()
    }

    async fn load_music_list() -> Result<Vec<String>> {
        let mut music = Vec::new();
        for entry in std::fs::read_dir(Self::asset_path("music"))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_string_lossy();
                music.push(name.to_string())
            }
        }

        Ok(music)
    }

    pub fn play_music(&self, file_name: &str) {
        match self.current_sink.replace(None) {
            Some((current_file, sink)) if current_file == file_name => {
                self.current_sink.replace(Some((current_file, sink)));
            }
            _ => {
                let sink = Sink::try_new(&self.stream_handle).unwrap();
                let file = BufReader::new(File::open(format!("./assets/music/{file_name}")).unwrap());
                let source = Decoder::new(file).unwrap().repeat_infinite();
                #[cfg(debug_assertions)]
                sink.set_volume(0.4);
                sink.append(source);

                self.current_sink.replace(Some((file_name.to_string(), sink)));
            }
        }
    }

    pub fn stop_music(&self) {
        self.current_sink.replace(None);
    }
}

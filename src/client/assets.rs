// use anyhow::anyhow;
// // use macroquad::prelude::*;
// // use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

// use notan::{draw::*, log, prelude::*, egui::{EguiRegisterTexture, TextureId}};

// pub struct DualTexture {
//     pub texture: Asset<Texture>,
//     pub egui: Option<TextureId>,
//     pub size : Option<(f32, f32)>
// }

// // TODO cleanup

// impl DualTexture {
//     pub fn new(texture: Asset<Texture>) -> Self {
//         Self {
//             texture,
//             egui: None,
//             size: None,
//         }
//     }

//     pub fn is_loaded(&self) -> bool {
//         self.texture.is_loaded() && self.egui.is_some()
//     }

//     pub fn tick(&mut self, gfx: &mut Graphics) {
//         if self.texture.is_loaded() {
//             if self.size.is_none() {
//                 self.size = self.texture.lock().map(|texture| texture.size())
//             }

//             if self.egui.is_none() {
//                 self.egui = self.texture.lock().map(|texture| gfx.egui_register_texture(&texture))
//             }
//         }
//     }
// }



// // #[derive(Clone)]
// // pub struct DualTexture {
// //     pub name: String,
// //     pub texture: Texture2D,
// //     pub egui: egui::TextureHandle,
// // }

// // impl DualTexture {
// //     fn from_image(name: &str, image: &Image) -> Self {
// //         let texture = Texture2D::from_image(image);
// //         texture.set_filter(FilterMode::Nearest);

// //         let mut egui: Option<egui::TextureHandle> = None;
// //         egui_macroquad::cfg(|ctx| {
// //             let size = [image.width(), image.height()];
// //             let image = egui::ColorImage::from_rgba_unmultiplied(size, &image.bytes);
// //             egui = Some(ctx.load_texture(name, image, egui::TextureFilter::Linear));
// //         });

// //         Self {
// //             name: name.to_string(),
// //             texture,
// //             egui: egui.expect("Could not convert texture to egui, impossible??"),
// //         }
// //     }
// // }

// const DEBUG_FONT: &[u8] = include_bytes!("../assets/LiberationMono-Regular.ttf");

// pub struct AssetCache {
//     pub sprites: Asset<Texture>,
//     pub font: Asset<Font>,
//     pub tileset: DualTexture,

//     pub debug_font: Font,

//     // pub font: Asset<Font>,
//     // tilesets: AssetList,
//     // pub tileset: RefCell<DualTexture>,
//     // pub sprites: Asset<Texture>,
//     // pub font: Font,

//     // _output_stream: OutputStream,
//     // stream_handle: OutputStreamHandle,

//     // music_list: Vec<String>,
//     // current_sink: RefCell<Option<(String, Sink)>>,
// }

// impl AssetCache {
//     pub fn load(assets: &mut Assets, gfx: &mut Graphics) -> anyhow::Result<Self> {
//         let sprites = assets.load_asset(&asset_path("sprites.png"))
//             .map_err(|e| anyhow!(e))?;
//         let tileset = assets.load_asset(&asset_path("tilesets/default.png"))
//             .map_err(|e| anyhow!(e))?;
//         let font = assets.load_asset(&asset_path("LiberationMono-Regular.ttf"))
//             .map_err(|e| anyhow!(e))?;

//         let debug_font = gfx.create_font(DEBUG_FONT)
//             .map_err(|e| anyhow!(e))?;

//         let tileset = DualTexture::new(tileset);

//         // let font = assets
//         //     .load_asset(&asset_path("LiberationMono-Regular.ttf"))
//         //     .map_err(|e| anyhow!(e))?;
//         // let sprites = assets.
//         //     load_asset(&asset_path("sprites.png"))
//         //     .map_err(|e| anyhow!(e))?;
//         // let sprites = load_image(&Self::asset_path_str("sprites.png")).await?;
//         //let sprites = DualTexture::from_image("sprites.png", &sprites);
//         // let font = load_ttf_font(&Self::asset_path_str("LiberationMono-Regular.ttf")).await?;

//         // let tilesets = Assets::load_tilesets().await?;
//         // let music_list = Assets::load_music_list().await?;

//         // // unwrap: Assets::load_tilesets ensures that at least "default.png" always exists
//         // let tileset = DualTexture::from_image("default.png", &tilesets["default.png"]);
//         // let (stream, stream_handle) = OutputStream::try_default()?;

//         Ok(Self {
//             debug_font,

//             sprites,
//             tileset,
//             font,

//             // font,
//             // sprites,
//             // tilesets,
//             // tileset: RefCell::new(tileset),
//             // music_list,
//             // current_sink: RefCell::new(None),
//             // sprites,
//             // font,
//             // _output_stream: stream,
//             // stream_handle,
//         })
//     }

//     pub fn tick(&mut self, gfx: &mut Graphics) {
//         self.tileset.tick(gfx);
//     }

//     pub fn is_loaded(&self) -> bool {
//         self.sprites.is_loaded() && self.font.is_loaded() && self.tileset.is_loaded()
//     }


//     // fn load_tilesets() -> Result<HashMap<String, Image>> {
//     //     // let mut tilesets = HashMap::new();
//     //     // let glob_path = Self::asset_path("tilesets/**/*.png").display().to_string();

//     //     // for entry in globwalk::glob(glob_path)? {
//     //     //     let entry = entry?;
//     //     //     let path = entry.path();
//     //     //     log::debug!("Loading tileset {}", path.display());
//     //     //     let image = load_image(&path.to_string_lossy()).await?;
//     //     //     let name = path.file_name().unwrap().to_string_lossy();
//     //     //     tilesets.insert(name.to_string(), image);
//     //     // }

//     //     // if !tilesets.contains_key("default.png") {
//     //     //     return Err(anyhow!(
//     //     //         "the file \"{}\" does not exist, but it is required to exist",
//     //     //         Self::asset_path("tilesets/default.png").display()
//     //     //     ));
//     //     // }

//     //     // Ok(tilesets)
//     // }

//     // pub fn tileset(&self) -> Ref<'_, DualTexture> {
//     //     self.tileset.borrow()
//     // }

//     // pub fn tilesets(&self) -> Vec<&str> {
//     //     self.tilesets.keys().map(|x| &**x).collect()
//     // }

//     // pub fn set_tileset(&self, name: &str) -> Result<()> {
//     //     let image = self
//     //         .tilesets
//     //         .get(name)
//     //         .ok_or_else(|| anyhow!("texture {name} not found"))?;
//     //     if self.tileset.borrow().name != name {
//     //         self.tileset.replace(DualTexture::from_image(name, image));
//     //     }
//     //     Ok(())
//     // }

//     // pub fn get_music(&self) -> Vec<String> {
//     //     self.music_list.clone()
//     // }

//     // async fn load_music_list() -> Result<Vec<String>> {
//     //     let prefix = PathBuf::from("./assets/music");
//     //     let music = globwalk::glob("assets/music/**/*.{mp3,ogg}")?
//     //         .into_iter()
//     //         .filter_map(Result::ok)
//     //         .map(|e| e.into_path())
//     //         .map(|p| p.strip_prefix(&prefix).unwrap().to_path_buf())
//     //         .map(|p| p.to_string_lossy().to_string())
//     //         .collect::<Vec<_>>();

//     //     log::debug!("{music:?}");

//     //     // let mut music = Vec::new();
//     //     // for entry in std::fs::read_dir(Self::asset_path("music"))? {
//     //     //     let entry = entry?;
//     //     //     let path = entry.path();
//     //     //     if path.is_file() {
//     //     //         let name = path.file_name().unwrap().to_string_lossy();
//     //     //         music.push(name.to_string());
//     //     //     }
//     //     // }

//     //     Ok(music)
//     // }

//     // pub fn toggle_music(&self, music: Option<&str>) {
//     //     if let Some(music) = music {
//     //         self.play_music(music);
//     //     } else {
//     //         self.stop_music();
//     //     }
//     // }

//     // fn play_music(&self, file_name: &str) {
//     //     let mut path = Self::asset_path("music");
//     //     path.push(file_name);

//     //     match self.current_sink.replace(None) {
//     //         Some((current_file, sink)) if current_file == file_name => {
//     //             self.current_sink.replace(Some((current_file, sink)));
//     //         }
//     //         _ => {
//     //             let sink = Sink::try_new(&self.stream_handle).unwrap();
//     //             let file = BufReader::new(File::open(path).unwrap());
//     //             let source = Decoder::new(file).unwrap().repeat_infinite();
//     //             #[cfg(debug_assertions)]
//     //             sink.set_volume(0.4);
//     //             sink.append(source);

//     //             self.current_sink.replace(Some((file_name.to_string(), sink)));
//     //         }
//     //     }
//     // }

//     // fn stop_music(&self) {
//     //     self.current_sink.replace(None);
//     // }
// }

// // The relative path for the example is different on browsers
// fn asset_path(path: &str) -> String {
//     common::client_path(format!("./assets/{path}"))
//         .to_string_lossy()
//         .to_string()
// }

// pub fn create_font_parser() -> AssetLoader {
//     AssetLoader::new().use_parser(parse_font).extension("ttf")
// }

// fn parse_font(id: &str, data: Vec<u8>, gfx: &mut Graphics) -> Result<Font, String> {
//     let font = gfx.create_font(&data)?;
//     log::debug!("Asset '{}' parsed as Font", id);
//     Ok(font)
// }

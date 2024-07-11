mod assets;
mod data;
mod network;
mod scene;
mod title;
mod game;
mod utils;

use std::path::PathBuf;

use ggegui::egui::load::SizedTexture;
use ggegui::Gui;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::{event, ContextBuilder};
use ggez::graphics::{self, Image};
use ggez::{Context, GameResult};
use ggez::glam::*;

use scene::Scene;
use title::TitleScene;

fn main() -> GameResult {
    env_logger::init();

    let mut cb = ContextBuilder::new("onyx_engine", "tylian")
        .window_setup(WindowSetup::default().title("Onyx Engine").vsync(false))
        .window_mode(WindowMode::default().dimensions(1600.0, 900.0));

    if let Ok(runtime) = std::env::var("RUNTIME_PATH") {
        let runtime = PathBuf::from(runtime).join("client");
        let resources = runtime.join("resources");

        println!("Setting runtime to {}", runtime.display());
        std::env::set_current_dir(runtime).unwrap();

        println!("Adding {resources:?} to path");
        cb = cb.add_resource_path(resources);
    }

    let (mut ctx, event_loop) = cb.build()?;

    let state = GameHandler::new(&mut ctx)?;
    event::run(ctx, event_loop, state)

    // let window_config = WindowConfig::new()
    //     .title("Onyx Engine")
    //     .vsync(true)
    //     //.multisampling(8)
    //     .size(1600, 900);

    // notan::init_with(setup)
    //     .add_config(window_config)
    //     .add_config(DrawConfig)
    //     .add_config(LogConfig::debug())
    //     .add_config(EguiConfig)
    //     .add_loader(assets::create_font_parser())
    //     .event(event)
    //     .update(update)
    //     .draw(draw)
    //     .build()
}

struct GameHandler {
    scene: Scene,
    state: GameState,
}

impl GameHandler {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        Ok(Self {
            scene: Scene::from(TitleScene::new(ctx)?),
            state: GameState::new(ctx)?
        })
    }
}

impl EventHandler for GameHandler {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.scene.update(ctx, &mut self.state)
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.scene.draw(ctx, &mut self.state)
    }

    fn quit_event(&mut self, ctx: &mut Context) -> GameResult<bool> {
       self.scene.event(ctx, &mut self.state, GameEvent::Quit).map(|_| false)
    }

    fn key_down_event(&mut self,
            _ctx: &mut Context,
            _input: ggez::input::keyboard::KeyInput,
            _repeated: bool,
        ) -> GameResult {
        // Override default so esc doesn't close game
        Ok(())
    }

    fn mouse_wheel_event(&mut self, ctx: &mut Context, mut x: f32, mut y: f32) -> GameResult {
        // ggez has a bug where it reports lines and pixels using the same x,y, without scaling the line amount
        // Try to detect it here, and scale it ourselves
        if x.abs() > 0.0 && x.abs() <= 2.0 {
            x *= 16.0;
        }

        if y.abs() > 0.0 && y.abs() <= 2.0 {
            y *= 16.0;
        }

        self.state.gui.input.mouse_wheel_event(x, y, ctx.keyboard.active_mods());
        Ok(())
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
        self.state.gui.input.text_input_event(character);
        self.scene.event(ctx, &mut self.state, GameEvent::TextInput(character))
    }
}

struct GameState {
    assets: AssetCache,
    gui: Gui,
}

impl GameState {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let mut gui = Gui::new(ctx);
        let assets = AssetCache::new(ctx, &mut gui)?;

        Ok(Self {
            assets,
            gui
        })
    }
}


#[derive(Clone, Copy, Eq, PartialEq)]
enum GameEvent {
    Quit,
    TextInput(char)
}


pub struct AssetCache {
    sprites: Image,
    tileset: Image,
    tileset_egui: SizedTexture,
}

impl AssetCache {
    fn new(ctx: &mut Context, gui: &mut Gui) -> GameResult<Self> {
        let sprites = graphics::Image::from_path(ctx, "/sprites.png")?;
        let tileset = graphics::Image::from_path(ctx, "/tilesets/default.png")?;

        let tileset_egui = gui.allocate_texture(tileset.clone());

        // let pixels = tileset.to_pixels(ctx)?;

        // let color_image = egui::ColorImage::from_rgba_unmultiplied(
        //     [tileset.width() as usize, tileset.height() as usize], 
        //     &pixels
        // );

        // let tileset_egui = gui.ctx().load_texture(
        //     "tileset",
        //     color_image,
        //     Default::default()
        // );

        Ok(Self {
            sprites,
            tileset,
            tileset_egui
        })
    }
}


// #[derive(AppState)]
// struct GameState {
//     current_state: Option<Box<dyn State>>,
//     next_state_fn: Option<SetupCallback>
// }

// impl GameState {
//     fn new(setup: SetupCallback) -> Self {
//         Self {
//             current_state: None,
//             next_state_fn: Some(setup),
//         }
//     }

//     /// Transition to the next state using next_state_fn, *must* be called before calling draw etc
//     fn next_state(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins) {
//         if let Some(next_state_fn) = self.next_state_fn.take() {
//             let mut ctx = SetupContext { app, assets, gfx, plugins };
//             self.current_state = Some(next_state_fn(&mut ctx));
//         }
//     }

//     fn prepare_state(&mut self, next_state_fn: Option<SetupCallback>) {
//         if next_state_fn.is_some() {
//             self.next_state_fn = next_state_fn;
//         }
//     }

//     fn update(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins) {
//         let mut ctx = UpdateContext { app, assets, plugins, next_state_fn: None };
//         self.current_state.as_mut()
//             .expect("somehow left state uninitialized")
//             .update(&mut ctx);

//         self.prepare_state(ctx.next_state_fn);
//     }

//     fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins) {
//         let mut ctx = DrawContext { app, assets, gfx, plugins, next_state_fn: None };
//         self.current_state.as_mut()
//             .expect("somehow left state uninitialized")
//             .draw(&mut ctx);
    
//         self.prepare_state(ctx.next_state_fn);
//     }

//     fn event(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, event: Event) {
//         let mut ctx = EventContext { app, assets, plugins, event, next_state_fn: None };
//         self.current_state.as_mut()
//             .expect("somehow left state uninitialized")
//             .event(&mut ctx);

//         self.prepare_state(ctx.next_state_fn);
//     }
// }

// fn setup(app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins) -> GameState {
//     let mut state = GameState::new(Box::new(TitleState::new_erased));
//     state.next_state(app, assets, gfx, plugins);

//     state
// }

// fn update(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut GameState) {
//     state.update(app, assets, plugins);
// }

// fn draw(app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut GameState) {
//     state.next_state(app, assets, gfx, plugins);
//     state.draw(app, assets, gfx, plugins);
// }


// fn event(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut GameState, event: Event) {
//     state.event(app, assets, plugins, event);
// }
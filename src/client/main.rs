mod data;
mod network;
mod scene;
mod title_scene;
mod game_scene;
mod utils;
mod ui;

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
use title_scene::TitleScene;

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

        Ok(Self {
            sprites,
            tileset,
            tileset_egui
        })
    }
}
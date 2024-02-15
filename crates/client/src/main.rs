mod assets;
mod data;
mod network;
mod scene;
mod title;
mod game;
mod utils;

use std::path::PathBuf;

use ggez::conf::WindowSetup;
use ggez::event::EventHandler;
use ggez::{event, ContextBuilder};
use ggez::graphics::{self, Image};
use ggez::{Context, GameResult};
use ggez::glam::*;

use scene::SceneStack;
use title::TitleScene;

fn main() -> GameResult {
    env_logger::init();

    let mut cb = ContextBuilder::new("onyx_engine", "tylian")
        .window_setup(WindowSetup::default().title("Onyx Engine").vsync(false));

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
    scenes: SceneStack<GameState, GameEvent>
}

impl GameHandler {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let state = GameState::new(ctx)?;
        let title_scene = TitleScene::new(ctx)?;
        let scenes = SceneStack::new(Box::new(title_scene), state);

        Ok(Self {
            scenes
        })
    }
}

impl EventHandler for GameHandler {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.scenes.update(ctx)
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.scenes.draw(ctx)
    }

    fn quit_event(&mut self, ctx: &mut Context) -> GameResult<bool> {
       self.scenes.event(ctx, GameEvent::Quit).map(|_| false)
    }

    fn key_down_event(&mut self,
            _ctx: &mut Context,
            _input: ggez::input::keyboard::KeyInput,
            _repeated: bool,
        ) -> GameResult {
        // Override default so esc doesn't close game
        Ok(())
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
        self.scenes.event(ctx, GameEvent::TextInput(character))
    }
}

struct GameState {
    assets: AssetCache,
}

impl GameState {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let assets = AssetCache::new(ctx)?;

        Ok(Self {
            assets,
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
}

impl AssetCache {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let sprites = graphics::Image::from_path(ctx, "/sprites.png")?;
        let tileset = graphics::Image::from_path(ctx, "/tilesets/default.png")?;

        Ok(Self {
            sprites,
            tileset
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
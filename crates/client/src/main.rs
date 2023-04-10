#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod data;
mod game;
mod network;
mod state;
mod title;
mod utils;

use notan::{draw::DrawConfig, egui::EguiConfig, log::LogConfig, prelude::*};

use state::{State, DrawContext, UpdateContext, SetupContext, SetupCallback, EventContext};
use title::TitleState;

#[notan_main]
fn main() -> Result<(), String> {
    let window_config = WindowConfig::new()
        .title("Onyx Engine")
        .vsync(true)
        //.multisampling(8)
        .size(1600, 900);

    notan::init_with(setup)
        .add_config(window_config)
        .add_config(DrawConfig)
        .add_config(LogConfig::debug())
        .add_config(EguiConfig)
        .add_loader(assets::create_font_parser())
        .event(event)
        .update(update)
        .draw(draw)
        .build()
}

#[derive(AppState)]
struct GameState {
    current_state: Option<Box<dyn State>>,
    next_state_fn: Option<SetupCallback>
}

impl GameState {
    fn new(setup: SetupCallback) -> Self {
        Self {
            current_state: None,
            next_state_fn: Some(setup),
        }
    }

    /// Transition to the next state using next_state_fn, *must* be called before calling draw etc
    fn next_state(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins) {
        if let Some(next_state_fn) = self.next_state_fn.take() {
            let mut ctx = SetupContext { app, assets, gfx, plugins };
            self.current_state = Some(next_state_fn(&mut ctx));
        }
    }

    fn prepare_state(&mut self, next_state_fn: Option<SetupCallback>) {
        if next_state_fn.is_some() {
            self.next_state_fn = next_state_fn;
        }
    }

    fn update(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins) {
        let mut ctx = UpdateContext { app, assets, plugins, next_state_fn: None };
        self.current_state.as_mut()
            .expect("somehow left state uninitialized")
            .update(&mut ctx);

        self.prepare_state(ctx.next_state_fn);
    }

    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins) {
        let mut ctx = DrawContext { app, assets, gfx, plugins, next_state_fn: None };
        self.current_state.as_mut()
            .expect("somehow left state uninitialized")
            .draw(&mut ctx);
    
        self.prepare_state(ctx.next_state_fn);
    }

    fn event(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, event: Event) {
        let mut ctx = EventContext { app, assets, plugins, event, next_state_fn: None };
        self.current_state.as_mut()
            .expect("somehow left state uninitialized")
            .event(&mut ctx);

        self.prepare_state(ctx.next_state_fn);
    }
}

fn setup(app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins) -> GameState {
    let mut state = GameState::new(Box::new(TitleState::new));
    state.next_state(app, assets, gfx, plugins);

    state
}

fn update(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut GameState) {
    state.update(app, assets, plugins);
}

fn draw(app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut GameState) {
    state.next_state(app, assets, gfx, plugins);
    state.draw(app, assets, gfx, plugins);
}


fn event(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut GameState, event: Event) {
    state.event(app, assets, plugins, event);
}
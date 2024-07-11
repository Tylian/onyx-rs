use ggez::GameResult;

use crate::{
    game_scene::GameScene, title_scene::TitleScene, GameEvent, GameState
};

//? manual enum dispatch
pub enum Scene {
    Title(TitleScene),
    Game(GameScene)
}

impl From<TitleScene> for Scene {
    fn from(value: TitleScene) -> Self {
        Self::Title(value)
    }
}

impl From<GameScene> for Scene {
    fn from(value: GameScene) -> Self {
        Self::Game(value)
    }
}

impl Scene {
    pub fn update(&mut self, ctx: &mut ggez::Context, state: &mut GameState) -> GameResult<()> {
        let transition = match self {
            Scene::Title(scene) => scene.update(ctx, state)?,
            Scene::Game(scene) => scene.update(ctx, state)?,
        };

        match transition {
            Transition::None => (), // noop
            Transition::Switch(next_scene) => *self = next_scene,
            Transition::Quit => ctx.request_quit(),
        }
        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut ggez::Context, state: &mut GameState) -> GameResult<()> {
        match self {
            Scene::Title(scene) => scene.draw(ctx, state),
            Scene::Game(scene) => scene.draw(ctx, state),
        }
    }

    pub fn event(&mut self, ctx: &mut ggez::Context, state: &mut GameState, event: GameEvent) -> GameResult<()> {
        match self {
            Scene::Title(scene) => scene.event(ctx, state, event),
            Scene::Game(scene) => scene.event(ctx, state, event),
        }
    }
}

#[allow(dead_code)]
pub enum Transition {
    None,
    Switch(Scene),
    Quit
}
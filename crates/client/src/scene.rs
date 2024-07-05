use ggez::GameResult;

pub enum SceneTransition<S, E> {
    None,
    Switch(Box<dyn Scene<S, E>>),
    Quit
}

impl<S, E> SceneTransition<S, E> {
    pub fn switch<Sc: Scene<S, E> + 'static>(scene: Sc) -> Self {
        Self::Switch(Box::new(scene))
    }
}

pub struct SceneStack<S, E> {
    pub state: S,
    scene: Box<dyn Scene<S, E>>
}

impl<S, E> SceneStack<S, E> {
    pub fn new(initial: Box<dyn Scene<S, E>>, state: S) -> Self {
        Self {
            state,
            scene: initial
        }
    }

    pub fn update(&mut self, ctx: &mut ggez::Context) -> GameResult<()> {
        match self.scene.update(ctx, &mut self.state)? {
            SceneTransition::None => (), // noop
            SceneTransition::Switch(next_scene) => self.scene = next_scene,
            SceneTransition::Quit => ctx.request_quit(),
        }
        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut ggez::Context) -> GameResult<()> {
        self.scene.draw(ctx, &mut self.state)
    }

    pub fn event(&mut self, ctx: &mut ggez::Context, event: E) -> GameResult<()> {
        self.scene.event(ctx, &mut self.state, event)
    }
}

//? Draw Scene is mutable because egui doesn't expose the output type
//? so I can't store it in update and then apply it in draw
pub trait Scene<S, E> {
    fn update(&mut self, ctx: &mut ggez::Context, state: &mut S) -> GameResult<SceneTransition<S, E>>;
    fn draw(&mut self, ctx: &mut ggez::Context, state: &mut S) -> GameResult;
    fn event(&mut self, ctx: &mut ggez::Context, state: &mut S, event: E) -> GameResult;
}
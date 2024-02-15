use ggez::{GameResult, GameError};

pub enum SceneTransition<S, E> {
    None,
    Pop,
    Push(Box<dyn Scene<S, E>>),
    Switch(Box<dyn Scene<S, E>>),
    Quit
}

impl<S, E> SceneTransition<S, E> {
    pub fn switch<Sc: Scene<S, E> + 'static>(scene: Sc) -> Self {
        Self::Switch(Box::new(scene))
    }

    pub fn push<Sc: Scene<S, E> + 'static>(scene: Sc) -> Self {
        Self::Push(Box::new(scene))
    }
}

pub struct SceneStack<S, E> {
    state: S,
    stack: Vec<Box<dyn Scene<S, E>>>
}

impl<S, E> SceneStack<S, E> {
    pub fn new(initial: Box<dyn Scene<S, E>>, state: S) -> Self {
        Self {
            state,
            stack: vec![initial]
        }
    }

    pub fn update(&mut self, ctx: &mut ggez::Context) -> GameResult<()> {
        let Some(scene) = self.stack.last_mut() else {
            return Err(GameError::CustomError("Attempted to update an empty SceneStack".to_string()));
        };

        match scene.update(ctx, &mut self.state)? {
            SceneTransition::None => (), // noop
            SceneTransition::Pop => {
                if self.stack.len() > 1 {
                    return Err(GameError::CustomError("Attempted to pop Scene off a stack with length 1".to_string()));
                }
                
                self.stack.pop();
            },
            SceneTransition::Push(next_scene) => self.stack.push(next_scene),
            SceneTransition::Switch(next_scene) => *scene = next_scene,
            SceneTransition::Quit => ctx.request_quit(),
        }
        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut ggez::Context) -> GameResult<()> {
        let Some(scene) = self.stack.last_mut() else {
            return Err(GameError::CustomError("Attempted to draw an empty SceneStack".to_string()));
        };

        scene.draw(ctx, &mut self.state)
    }

    pub fn event(&mut self, ctx: &mut ggez::Context, event: E) -> GameResult<()> {
        let Some(scene) = self.stack.last_mut() else {
            return Err(GameError::CustomError("Attempted to send an event to an empty SceneStack".to_string()));
        };

        scene.event(ctx, &mut self.state, event)
    }
}

//? Draw Scene is mutable because egui doesn't expose the output type
//? so I can't store it in update and then apply it in draw
pub trait Scene<S, E> {
    fn update(&mut self, ctx: &mut ggez::Context, state: &mut S) -> GameResult<SceneTransition<S, E>>;
    fn draw(&mut self, ctx: &mut ggez::Context, state: &mut S) -> GameResult;
    fn event(&mut self, ctx: &mut ggez::Context, state: &mut S, event: E) -> GameResult;
}
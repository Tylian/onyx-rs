use ggez::{event::EventHandler, GameResult, GameError};

pub enum SceneTransition<E> {
    None,
    Pop,
    Push(Box<dyn Scene<E>>),
    Switch(Box<dyn Scene<E>>),
    Quit
}

impl<E> SceneTransition<E> {
    pub fn switch<S: Scene<E> + 'static>(scene: S) -> Self {
        Self::Switch(Box::new(scene))
    }

    pub fn push<S: Scene<E> + 'static>(scene: S) -> Self {
        Self::Push(Box::new(scene))
    }
}

pub struct SceneStack<E> {
    // state: S
    stack: Vec<Box<dyn Scene<E>>>
}

impl<E> SceneStack<E> {
    pub fn new(initial: Box<dyn Scene<E>>) -> Self {
        Self {
            stack: vec![initial]
        }
    }

    pub fn update(&mut self, ctx: &mut ggez::Context) -> GameResult<()> {
        let Some(scene) = self.stack.last_mut() else {
            return Err(GameError::CustomError("Attempted to update an empty SceneStack".to_string()));
        };

        match scene.update(ctx)? {
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

        scene.draw(ctx)
    }

    pub fn event(&mut self, ctx: &mut ggez::Context, event: E) -> GameResult<()> {
        let Some(scene) = self.stack.last_mut() else {
            return Err(GameError::CustomError("Attempted to send an event to an empty SceneStack".to_string()));
        };

        scene.event(ctx, event)
    }
}

//? Draw Scene is mutable because egui doesn't expose the output type
//? so I can't store it in update and then apply it in draw
pub trait Scene<E> {
    fn update(&mut self, ctx: &mut ggez::Context) -> GameResult<SceneTransition<E>>;
    fn draw(&mut self, ctx: &mut ggez::Context) -> GameResult;
    fn event(&mut self, ctx: &mut ggez::Context, event: E) -> GameResult;
}
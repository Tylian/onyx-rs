use notan::{prelude::*, draw::Draw};

pub type SetupCallback = Box<dyn FnOnce(&mut SetupContext) -> Box<dyn State>>;

pub struct DrawContext<'a> {
    pub app: &'a mut App,
    pub assets: &'a mut Assets,
    pub gfx: &'a mut Graphics,
    pub plugins: &'a mut Plugins,
    pub next_state_fn: Option<SetupCallback>,
}

pub struct UpdateContext<'a> {
    pub app: &'a mut App,
    pub assets: &'a mut Assets,
    pub plugins: &'a mut Plugins,
    pub next_state_fn: Option<SetupCallback>,
}

pub struct SetupContext<'a> {
    pub app: &'a mut App,
    pub assets: &'a mut Assets,
    pub gfx: &'a mut Graphics,
    pub plugins: &'a mut Plugins,
}

pub struct TransitionContext<'a> {
    pub app: &'a App,
    pub assets: &'a Assets,
    pub plugins: &'a Plugins,
}

pub struct EventContext<'a> {
    pub app: &'a mut App,
    pub assets: &'a mut Assets,
    pub plugins: &'a mut Plugins,
    pub event: Event,
    pub next_state_fn: Option<SetupCallback>,
}

//? Draw state is mutable because egui doesn't expose the output type
//? so I can't store it in update and then apply it in draw
pub trait State {
    fn draw(&mut self, ctx: &mut DrawContext);
    fn update(&mut self, ctx: &mut UpdateContext);
    fn event(&mut self, ctx: &mut EventContext);
    fn enter(&mut self, ctx: &mut TransitionContext) {}
    fn exit(&mut self, ctx: &mut TransitionContext) {}
}
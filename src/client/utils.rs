use ggez::{context::Has, glam::Vec2, graphics::{Canvas, Color, DrawParam, Drawable, GraphicsContext, Rect, Text, Transform}};

/// Shortcut that is equivalent to if !$cond { return Err($err); }
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}

pub fn ping_pong(t: f32, frames: u32) -> u32 {
    let steps = frames * 2 - 2;
    let frame = (steps as f32 * t) as u32;
    if frame >= frames {
        steps - frame
    } else {
        frame
    }
}

pub struct OutlinedText<'a> {
    inner: &'a Text,
    outline_color: Color
}

impl<'a> OutlinedText<'a> {
    pub fn new(text: &'a Text) -> Self {
        Self {
            inner: text,
            outline_color: Color::new(0.0, 0.0, 0.0, 0.5)
        }
    }
    #[allow(dead_code)]
    pub fn outline_color(self, color: impl Into<Color>) -> Self {
        Self {
            outline_color: color.into(),
            ..self
        }
    }
}

#[allow(clippy::clone_on_copy)] // explicit clone for readability + intent
impl<'a> Drawable for OutlinedText<'a> {
    fn draw(&self, canvas: &mut Canvas, param: impl Into<DrawParam>) {
        const OFFSETS: &[Vec2] = &[
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, -0.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
        ];

        let param: DrawParam = param.into();
        let Transform::Values { dest, .. } = param.transform else {
            return canvas.draw(self.inner, param);
        };
        let dest = Vec2::from(dest);

        for offset in OFFSETS {
            let param = param.clone()
                .dest(dest + *offset)
                .color(self.outline_color);
            canvas.draw(self.inner, param);
        }
    
        canvas.draw(self.inner, param);
    }

    fn dimensions(&self, gfx: &impl Has<GraphicsContext>) -> Option<Rect> {
        let rect = self.inner.dimensions(gfx);
        rect.map(|inner| Rect::new(
            inner.x - 1.0,
            inner.y - 1.0,
            inner.w + 2.0,
            inner.h + 2.0
        ))
    }
}
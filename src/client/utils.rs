// use notan::{draw::*, math::*, prelude::Color};
use ggez::{context::Has, glam::Vec2, graphics::{Canvas, Color, DrawParam, Drawable, GraphicsContext, Rect, Text, Transform}};

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

// // idk what to do with this
// pub fn draw_text_shadow<'a, F>(draw: &mut Draw, font: &'a Font, text: &'a str, position: Vec2, builder: F)
//     where F: Fn(&mut DrawBuilder<TextSection<'a>>)
// {
//     // builder(draw.text(font, text));
//     // draw.text(font, text);
//     {
//         let shadow_position = position + glam::vec2(1.0, 1.0);
//         let mut text_section = draw.text(font, text);
//         builder(&mut text_section);

//         text_section.position(shadow_position.x, shadow_position.y)
//         .color(Color::from_rgba(0.0, 0.0, 0.0, 0.5));
//     }

//     let mut text_section = draw.text(font, text);
//     builder(&mut text_section);
//     text_section.position(position.x, position.y);
// }

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

// pub fn draw_text_outline<FP, FT>(canvas: &mut Canvas, text_builder: FT, param_builder: FP)
//     where FT: FnOnce() -> Text,
//           FP: Fn(Vec2, bool) -> DrawParam
// {
//     let outlines = &[
//         Vec2::new(1.0, 0.0),
//         Vec2::new(-1.0, 0.0),
//         Vec2::new(0.0, 1.0),
//         Vec2::new(0.0, -1.0),
//         Vec2::new(-1.0, -0.0),
//         Vec2::new(-1.0, 1.0),
//         Vec2::new(1.0, -1.0),
//         Vec2::new(1.0, 1.0),
//     ];

//     let text = text_builder();

//     for outline in outlines {
//         canvas.draw(&text, param_builder(*outline, true));
//     }

//     canvas.draw(&text, param_builder(Vec2::ZERO, false));
// }

// pub fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
//     Rect {
//         x, y, width, height
//     }
// }

// pub trait RectExt {
//     fn left(&self) -> f32;
//     fn right(&self) -> f32;
//     fn top(&self) -> f32;
//     fn bottom(&self) -> f32;
//     fn center(&self) -> Vec2;
//     fn size(&self) -> Vec2;
//     fn contains(&self, point: Vec2) -> bool;
//     fn overlaps(&self, other: &Rect) -> bool;
// }

// impl RectExt for Rect {
//     fn left(&self) -> f32 {
//         self.x
//     }

//     fn right(&self) -> f32 {
//         self.x + self.width
//     }

//     fn top(&self) -> f32 {
//         self.y
//     }

//     fn bottom(&self) -> f32 {
//         self.y + self.height
//     }

//     fn center(&self) -> Vec2 {
//         vec2(self.x + self.width * 0.5, self.y + self.height * 0.5)
//     }

//     fn size(&self) -> Vec2 {
//         vec2(self.width, self.height)
//     }

//     fn contains(&self, point: Vec2) -> bool {
//         point.x >= self.left()
//             && point.x < self.right()
//             && point.y < self.bottom()
//             && point.y >= self.top()
//     }

//     fn overlaps(&self, other: &Rect) -> bool {
//         self.left() <= other.right()
//             && self.right() >= other.left()
//             && self.top() <= other.bottom()
//             && self.bottom() >= other.top()
//     }
// }
// use notan::{draw::*, math::*, prelude::Color};

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}

pub fn ping_pong(t: f32, frames: u16) -> u16 {
    let steps = frames * 2 - 2;
    let frame = (steps as f32 * t) as u16;
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

// pub fn draw_text_outline<'a, F>(draw: &mut Draw, font: &'a Font, text: &'a str, position: Vec2, builder: F)
//     where F: Fn(&mut DrawBuilder<TextSection<'a>>)
// {
//     let outlines = &[
//         glam::vec2(1.0, 0.0),
//         glam::vec2(-1.0, 0.0),
//         glam::vec2(0.0, 1.0),
//         glam::vec2(0.0, -1.0),
//         glam::vec2(-1.0, -0.0),
//         glam::vec2(-1.0, 1.0),
//         glam::vec2(1.0, -1.0),
//         glam::vec2(1.0, 1.0),
//     ];

//     for outline in outlines {
//         let mut text = draw.text(font, text);
//         builder(&mut text);

//         let position = position + *outline;
//         text.position(position.x, position.y)
//             .color(Color::from_rgba(0.0, 0.0, 0.0, 0.5));
//     }

//     let mut text = draw.text(font, text);
//     builder(&mut text);
//     text.position(position.x, position.y);
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
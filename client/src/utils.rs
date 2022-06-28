use macroquad::prelude::*;

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}

pub fn ping_pong(t: f64, frames: u16) -> u16 {
    let steps = frames * 2 - 2;
    let frame = (steps as f64 * t) as u16;
    if frame >= frames {
        steps - frame
    } else {
        frame
    }
}

pub fn draw_text_shadow(text: &str, position: Vec2, params: TextParams) {
    let shadow_position = position + glam::vec2(1.0, 1.0);
    draw_text_ex(
        text,
        shadow_position.x,
        shadow_position.y,
        TextParams {
            color: Color::new(0.0, 0.0, 0.0, 0.5),
            ..params
        },
    );

    draw_text_ex(text, position.x, position.y, params);
}

pub fn draw_text_outline(text: &str, position: Vec2, params: TextParams) {
    let outlines = &[
        glam::vec2(1.0, 0.0),
        glam::vec2(-1.0, 0.0),
        glam::vec2(0.0, 1.0),
        glam::vec2(0.0, -1.0),
        glam::vec2(-1.0, -0.0),
        glam::vec2(-1.0, 1.0),
        glam::vec2(1.0, -1.0),
        glam::vec2(1.0, 1.0),
    ];

    let outline_param = TextParams {
        color: Color::new(0.0, 0.0, 0.0, 0.5),
        ..params
    };

    for outline in outlines {
        let position = position + *outline;
        draw_text_ex(text, position.x, position.y, outline_param);
    }

    draw_text_ex(text, position.x, position.y, params);
}

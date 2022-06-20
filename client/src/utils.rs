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
    let outlines = &[
        (1.0, 0.0).into(),
        (-1.0, 0.0).into(),
        (0.0, 1.0).into(),
        (0.0, -1.0).into(),
        (-1.0, -0.0).into(),
        (-1.0, 1.0).into(),
        (1.0, -1.0).into(),
        (1.0, 1.0).into(),
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
use opengl_graphics::GlGraphics;
use piston::input::RenderArgs;

use crate::AUDIO_BUFFER;

pub struct Waveform {
    pub gl: GlGraphics,
    pub radius: f64,
}

impl Waveform {
    pub fn new(gl: GlGraphics, radius: f64) -> Self {
        Self {
            gl,
            radius,
        }
    }
    pub fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let (x_size, y_size) = (args.window_size[0], args.window_size[1]);
        let buffer = AUDIO_BUFFER.lock().unwrap().get_vec();
        let buffer_length = buffer.len();
        let mut buffer_iter = buffer.iter();
        let mut previous = match buffer_iter.next() {
            Some(v) => v,
            None => return,
        };
        let radius = self.radius;
        let mut count = 1;
        self.gl.draw(args.viewport(), |c, gl| {
            clear([0.0, 0.0, 0.0, 0.0], gl);

            for v in buffer_iter {
                let x1 = (count as f64 - 1_f64) * x_size / buffer_length as f64;
                let y1 = y_size / 2_f64 * -previous as f64 + y_size / 2_f64;
                let x2 = (count as f64) * x_size / buffer_length as f64;
                let y2 = y_size / 2_f64 * -v as f64 + y_size / 2_f64;
                Line {
                    color: [1.0, 1.0, 1.0, 1.0],
                    radius: radius,
                    shape: line::Shape::Round,
                }.draw([x1, y1, x2, y2], &c.draw_state, c.transform, gl);

                previous = v;
                count += 1;
            }
        });
        
    }
}
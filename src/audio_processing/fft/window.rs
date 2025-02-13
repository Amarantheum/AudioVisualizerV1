use std::f32::consts::PI;

pub type RealWindowFn = fn(&mut [f32]);

pub fn null_window_fn(buf: &mut [f32]) {}

pub trait WindowFunction {
    // applies window function to the buffer
    fn real_window(buffer: &mut [f32]);
}

pub struct BlackmanHarris;

impl WindowFunction for BlackmanHarris {
    fn real_window(buffer: &mut [f32]) {
        let size = buffer.len() as f32;
        for i in 0..buffer.len() {
            buffer[i] = buffer[i] * (
                0.35875 
                - 0.48829 * ((2 * i) as f32 * PI / size).cos() 
                + 0.14128 * ((4 * i) as f32 * PI / size).cos() 
                - 0.01168 * ((6 * i) as f32 * PI / size).cos()
            );
        }
    }
}

pub struct Rectangular;

impl WindowFunction for Rectangular {
    fn real_window(buffer: &mut [f32]) {
        ()
    }
}
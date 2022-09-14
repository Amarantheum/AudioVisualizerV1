use std::f32::consts::PI;

pub trait WindowFunction {
    // applies window function to the buffer
    fn window(buffer: &mut [f32]);
}

pub struct BlackmanHarris;

impl WindowFunction for BlackmanHarris {
    fn window(buffer: &mut [f32]) {
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


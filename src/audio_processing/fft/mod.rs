use std::sync::Arc;
use rustfft::algorithm::Radix4;

pub mod window;

pub struct FftCalculator {
    fft_planner: Radix4<f32>,
    // the size passed to create the 
    given_size: usize,
    zero_pad_length: usize,
    actual_size: usize,
}

impl FftCalculator {
    /*pub fn new() -> Self {
        Self {

        }
    }*/
}
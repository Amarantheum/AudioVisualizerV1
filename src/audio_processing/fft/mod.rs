use std::sync::Arc;
use rustfft::algorithm::Radix4;
use std::error::Error;
use rustfft::num_complex::Complex;
use rustfft::Fft;
use window::RealWindowFn;

pub mod window;

pub struct FftCalculator {
    fft_planner: Radix4<f32>,
    // the size passed to create the fft planner
    pub size: usize,
    pub zero_pad_length: usize,
    internal_buf: Vec<f32>
}

impl FftCalculator {
    pub fn new(size: usize, zero_pad_length: usize) -> Result<Self, Box<dyn Error>> {
        if (size + zero_pad_length).is_power_of_two() {
            return Err("failed to create fft calculator because size and zero_pad_length do not add to power of 2".into());
        }
        Ok(
            Self {
                fft_planner: Radix4::new(size + zero_pad_length, rustfft::FftDirection::Forward),
                size,
                zero_pad_length,
                internal_buf: vec![0_f32; size],
            }
        )
    }

    pub fn real_fft(&mut self, samples: &[f32], window_fn: RealWindowFn) -> Vec<Complex<f32>>
    {
        assert!(samples.len() == self.size);
        for i in 0..samples.len() {
            self.internal_buf[i] = samples[i];
        }
        window_fn(&mut self.internal_buf[..]);
        let mut out = Vec::with_capacity(self.size + self.zero_pad_length);
        for i in 0..samples.len() {
            out.push(Complex::<f32>::new(self.internal_buf[i], 0.0));
        }
        for _ in 0..self.zero_pad_length {
            out.push(Complex::<f32>::new(0.0, 0.0));
        }
        self.fft_planner.process(&mut out[..]);
        out
    }

    /// Get the amount to multiply the magnitude of each frequency bin by based on the amount of zero padding in the calculator
    #[inline]
    pub fn zero_pad_scale_factor(&self) -> f32 {
        (self.size as f32 + self.zero_pad_length as f32) / self.size as f32
    }
}
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
        if !(size + zero_pad_length).is_power_of_two() {
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

#[inline]
pub fn complex_to_vec4_arr(mut comp_buf: Vec<Complex<f32>>) -> Vec<[f32; 4]> {
    if comp_buf.len() & 1 != 0 {
        comp_buf.push(Complex::new(0.0, 0.0));
    }
    let (ptr, len, cap) = comp_buf.into_raw_parts();
    unsafe {
        Vec::from_raw_parts(ptr as *mut [f32; 4], len / 2, cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amplitude() {
        let mut fft = FftCalculator::new(8, 0).unwrap();
        let mut samples = [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1_f32, 1.0];
        let mut comp_buf = fft.real_fft(&samples[..], window::null_window_fn);
        let max_amp = comp_buf.iter().map(|x| x.norm()).fold(0.0, |a: f32, b: f32| a.max(b));
        let max_index = comp_buf.iter().position(|x| x.norm() == max_amp).unwrap();
        println!("max_amp: {:?}", max_amp);
        println!("max_index: {:?}", max_index);
        println!("comp_buf: {:?}", comp_buf);
    }
}
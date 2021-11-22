use opengl_graphics::GlGraphics;
use piston::input::RenderArgs;
use std::sync::Arc;
use rustfft::Fft;
use std::ops::Range;

use crate::{AUDIO_BUFFER, SAMPLE_RATE, BUFFER_SIZE};
use rustfft::{FftPlanner, num_complex::Complex};

pub struct Spectrum {
    pub gl: GlGraphics,
    pub radius: f64,
    pub fft: Arc<dyn Fft<f32>>,
    // x size, y size
    pub window_size: [f64; 2],
    pub state: Vec<f32>,
    pixel_bins: Vec<Range<usize>>,
    pub falloff: f32,
    resolution: f32,
    color: [f32; 4],
}

impl Spectrum {
    pub fn new(gl: GlGraphics, color: [f32; 4], radius: f64, falloff: f32, window_size: [f64; 2], resolution: f32) -> Self {
        Spectrum {
            gl,
            radius,
            fft: FftPlanner::new().plan_fft_forward(BUFFER_SIZE),
            window_size,
            state: vec![],
            pixel_bins: vec![],
            falloff,
            resolution,
            color,
        }
    }

    pub fn get_x(bin: usize, x_size: f64, range: f32) -> f32 {
        (Self::get_hertz(bin).log10() - 20_f32.log10()) * x_size as f32 / range
    }

    pub fn get_y(value: f32, y_size: f64) -> f32 {
        y_size as f32 - ((value + 0.5) * y_size as f32 / 4_f32)
    }

    // AUDIO_BUFFER size must be > 2
    pub fn render_resize(&mut self, args: &RenderArgs) {
        use graphics::*;
        let mut audio_buffer = AUDIO_BUFFER.lock().unwrap().get_vec();
        let [x_size, y_size] =  self.window_size;

        let audio_zero = if audio_buffer.iter().all(|v| v == &0_f32) { true } else { false };
        let real_buffer: Vec<f32>;
        let buff_size = Self::get_upper_cutoff_point() + 1;
        if !audio_zero {
            let zeros: Vec<Complex<f32>> = vec![Complex { re: 0_f32, im: 0_f32}; BUFFER_SIZE / 2];
            Self::blackman_harris(&mut audio_buffer);
            let mut complex_buffer: Vec<Complex<f32>> = audio_buffer.into_iter().map(float_to_complex).chain(zeros).collect();
            self.fft.process(&mut complex_buffer);
            real_buffer = complex_buffer[..buff_size].to_vec().into_iter().map(complex_to_float).collect();
        } else {
            real_buffer = vec![-6.0; buff_size];
        }
        let mut key_vec: Vec<[f64; 2]> = Vec::with_capacity((x_size as f32 / self.resolution) as usize);
        let mut state = Vec::with_capacity((x_size as f32 / self.resolution) as usize);
        let mut pixel_bins = Vec::with_capacity((x_size as f32 / self.resolution) as usize);
        
        let mut buffer_iter = real_buffer.iter().map(|v| v.log10()).enumerate();
        let _dc = buffer_iter.next().unwrap();
        let range = 20000_f32.log10() - 20_f32.log10();

        let (i, v) = match buffer_iter.next() { Some(vv) => vv, None => panic!("Buffer length must be > 2")};
        let mut x = Self::get_x(i, x_size, range);
        let mut cur_x = (x / self.resolution as f32).floor() * self.resolution as f32;
        let mut y = Self::get_y(v, y_size);
        loop {
            let mut max_x = x;
            let mut max_y = y; // min because lower y = higher
            let (i, v) = match buffer_iter.next() {
                Some(vv) => vv,
                None => break,
            };
            x = Self::get_x(i, x_size, range);
            y = Self::get_y(v, y_size);
            let bin_range_min = i - 1;
            let mut tmp_bin = i;
            while x - cur_x < self.resolution as f32 {
                if y < max_y {
                    max_x = x;
                    max_y = y;
                }
                let (i, v) = match buffer_iter.next() {
                    Some(vv) => vv,
                    None => break,
                };
                x = Self::get_x(i, x_size, range);
                y = Self::get_y(v, y_size);
                tmp_bin = i;
            }
            state.push(max_y);
            pixel_bins.push(bin_range_min..tmp_bin);
            key_vec.push([max_x as f64, max_y as f64]);
            cur_x = (x / self.resolution as f32).floor() * self.resolution as f32;
        }

        self.state = state;
        self.pixel_bins = pixel_bins;

        let radius = self.radius;
        let color = self.color;
        self.gl.draw(args.viewport(), |c, gl| {
            clear([0.0, 0.0, 0.0, 0.0], gl);
            let mut point_iter = key_vec.iter();
            let [mut prev_x, mut prev_y] = point_iter.next().unwrap();
            while let Some([x, y]) = point_iter.next() {
                Line {
                    color: color,
                    radius: radius,
                    shape: line::Shape::Round,
                }.draw([prev_x, prev_y, *x, *y], &c.draw_state, c.transform, gl);

                prev_x = *x;
                prev_y = *y;
            }
        });
        
        
    }

    // TODO: centralize similar code between render and render_resize
    pub fn render(&mut self, args: &RenderArgs) {
        let (x_size, y_size) = (args.window_size[0], args.window_size[1]);
        if x_size != self.window_size[0] || y_size != self.window_size[1] || self.state.len() == 0 {
            self.window_size = [x_size, y_size];
            self.render_resize(args);
        } else {
            use graphics::*;
            let mut audio_buffer = AUDIO_BUFFER.lock().unwrap().get_vec();
            let [x_size, y_size] =  self.window_size;

            let audio_zero = if audio_buffer.iter().all(|v| v == &0_f32) { true } else { false };
            let real_buffer: Vec<f32>;
            let buff_size = Self::get_upper_cutoff_point() + 1;
            if !audio_zero {
                let zeros: Vec<Complex<f32>> = vec![Complex { re: 0_f32, im: 0_f32}; BUFFER_SIZE / 2];
                Self::blackman_harris(&mut audio_buffer);
                let mut complex_buffer: Vec<Complex<f32>> = audio_buffer.into_iter().map(float_to_complex).chain(zeros).collect();
                self.fft.process(&mut complex_buffer);
                real_buffer = complex_buffer[..buff_size].to_vec().into_iter().map(complex_to_float).collect();
            } else {
                real_buffer = vec![-6.0; buff_size];
            }
            let mut key_vec: Vec<[f64; 2]> = Vec::with_capacity((x_size as f32 / self.resolution) as usize);
        
            let log_20 = 20_f32.log10();
            let range = 20000_f32.log10() - log_20;
            for (i, bins) in self.pixel_bins.iter().enumerate() {
                let mut max_bin = 0;
                let mut tmp_max_y = f32::MIN;
                for bin in bins.clone() {
                    let value = real_buffer[bin];
                    if value > tmp_max_y {
                        max_bin = bin;
                        tmp_max_y = value;
                    }
                }
                let max_y = Self::get_y(tmp_max_y.log10(), y_size);
                if max_y <= self.state[i] + self.falloff * y_size as f32 {
                    self.state[i] = max_y;
                } else {
                    self.state[i] = (self.state[i] + self.falloff * y_size as f32).min(y_size as f32 + 100_f32);
                }
                let max_x = Self::get_x(max_bin, x_size, range);
                key_vec.push([max_x as f64, self.state[i] as f64]);
            }

            let radius = self.radius;
            let color = self.color;
            self.gl.draw(args.viewport(), |c, gl| {
                clear([0.0, 0.0, 0.0, 0.0], gl);
                let mut point_iter = key_vec.iter();
                let [mut prev_x, mut prev_y] = point_iter.next().unwrap();
                while let Some([x, y]) = point_iter.next() {
                    Line {
                        color: color,
                        radius: radius,
                        shape: line::Shape::Round,
                    }.draw([prev_x, prev_y, *x, *y], &c.draw_state, c.transform, gl);

                    prev_x = *x;
                    prev_y = *y;
                }
            });
        }
    }

    fn get_hertz(bin: usize) -> f32 {
        unsafe { SAMPLE_RATE / BUFFER_SIZE as f32 * bin as f32 }
    }

    fn get_upper_cutoff_point() -> usize {
        unsafe { (20000 * BUFFER_SIZE) / SAMPLE_RATE as usize }
    }

    #[allow(dead_code)]
    fn get_lower_cutoff_point() -> usize {
        unsafe { (20 * BUFFER_SIZE) / SAMPLE_RATE as usize }
    }

    #[allow(dead_code)]
    // applies hann window function to BUFFER_SIZE / 2 elements from the front
    fn hann(buffer: &mut Vec<f32>) {
        let buffer_size = BUFFER_SIZE as f32 / 2_f32;
        for i in 0..BUFFER_SIZE / 2 {
            buffer[i] = buffer[i] * ((i as f32 * 2_f32 * std::f32::consts::PI / buffer_size).cos() / -2_f32 + 0.5);
        }
    }

    // applies blackman-harris window function to BUFFER_SIZE / 2 elements from the front
    fn blackman_harris(buffer: &mut Vec<f32>) {
        use std::f32::consts::PI;
        let buffer_size = (BUFFER_SIZE >> 1) as f32;
        for i in 0..BUFFER_SIZE / 2 {
            buffer[i] = buffer[i] * (
                0.35875 
                - 0.48829 * ((2 * i) as f32 * PI / (buffer_size - 1.0)).cos() 
                + 0.14128 * ((4 * i) as f32 * PI / (buffer_size - 1.0)).cos() 
                - 0.01168 * ((6 * i) as f32 * PI / (buffer_size - 1.0)).cos()
            );
        }
    }

    #[allow(dead_code)]
    fn local_avg_smooth(values: &mut Vec<f32>) {
        let mut previous = values[0];
        values[0] = (values[0] + values[1]) / 2_f32;
        for i in 1..values.len() - 1 {
            let tmp = previous;
            previous = values[i];
            values[i] = (values[i] + tmp + values[i + 1]) / 3_f32;
        }
        let last_index = values.len() - 1;
        values[last_index] = (values[last_index] + previous) / 2_f32;
    }
}

fn complex_to_float(c: Complex<f32>) -> f32 {
    (c.re.powf(2_f32) + c.im.powf(2_f32)).sqrt()
}

fn float_to_complex(f: f32) -> Complex<f32> {
    Complex { re: f, im: 0_f32 }
}


use plotters::prelude::*;

// use plotters to plot the given vector of floats. Useful for testing.
#[allow(dead_code)]
fn plot_frequencies_real(vec: &Vec<f32>, frames: usize) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new("./waveform.png", (4000, 1080)).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.margin(10, 10, 10, 10);
    // After this point, we should be able to draw construct a chart context
    let mut chart = ChartBuilder::on(&root)
        // Set the size of the label region
        .x_label_area_size(20)
        .y_label_area_size(40)
        // Finally attach a coordinate on the drawing area and make a chart context
        .build_cartesian_2d(0f32..frames as f32, -1_f32..1_f32)?;

    // Then we can draw a mesh
    chart
        .configure_mesh()
        // We can customize the maximum number of labels allowed for each axis
        .x_labels(5)
        .y_labels(5)
        // We can also change the format of the label text
        .y_label_formatter(&|x| format!("{:.3}", x))
        .draw()?;

    // And we can draw something in the drawing area
    let series = vec.iter().enumerate().map(|v| (v.0 as f32, *v.1)).collect::<Vec<(f32, f32)>>();
    chart.draw_series(LineSeries::new(
        series.clone(),
        &RED,
    ))?;
    Ok(())
}
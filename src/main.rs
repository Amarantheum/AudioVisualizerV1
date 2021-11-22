#[macro_use]
extern crate lazy_static;

use cpal::traits::{HostTrait, StreamTrait};
use system_audio::capture_output_audio;
use std::sync::Mutex;

use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::{EventSettings, Events};
use piston::input::{RenderEvent, UpdateEvent};
use piston::window::WindowSettings;

use audio_graphics::waveform::Waveform;
use audio_graphics::spectrum::Spectrum;
use ring_buffer::RingBuffer;

mod system_audio;
mod audio_graphics;
mod ring_buffer;

static mut SAMPLE_RATE: f32 = 48000_f32;
const BUFFER_SIZE: usize = 32768;

lazy_static! {
    static ref AUDIO_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());
    static ref WAVE: Mutex<Vec<f32>> = Mutex::new(Vec::new());
}

fn main() {

    let host = cpal::default_host();
    let device = host.default_output_device().expect("no default output device available");
    let stream = capture_output_audio(&device).unwrap();
    stream.play().unwrap();

    let opengl = OpenGL::V3_2;
    let window_size = [1000, 500];
    let mut window: Window = WindowSettings::new("Audio Spectrum", window_size)
        .samples(4)
        .graphics_api(opengl)
        .exit_on_esc(true)
        .decorated(true)
        .transparent(true)
        .build()
        .unwrap();

    // uncomment this and comment out the graph definition below to get a waveform visualizer
    /*let mut graph = Waveform {
        gl: GlGraphics::new(opengl),
        radius: 0.5_f64,
    };*/
    let mut graph = Spectrum::new(GlGraphics::new(opengl), [0.619, 0.733, 1.0, 1.0], 1_f64, 0.015, [1000_f64, 500_f64], 1_f32);
    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            graph.render(&args);
        }

        if let Some(_args) = e.update_args() {}
    }
}
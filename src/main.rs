#![feature(vec_into_raw_parts)]
// uncomment for release
//#![windows_subsystem = "windows"]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate glium;

use cpal::traits::{HostTrait, StreamTrait};
use system_audio::capture_output_audio;
use parking_lot::Mutex;
use glium::{Surface, VertexBuffer, IndexBuffer};
use graphics::vertex::Vertex;

//use audio_graphics::waveform::Waveform;
//use audio_graphics::spectrum::Spectrum;
use ring_buffer::RingBuffer;
use graphics::Spectrum;

mod system_audio;
mod audio_graphics;
mod ring_buffer;
mod graphics;
mod audio_processing;

static mut SAMPLE_RATE: f32 = 48000_f32;
const BUFFER_SIZE: usize = 32768;

lazy_static! {
    static ref AUDIO_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());
    static ref WAVE: Mutex<Vec<f32>> = Mutex::new(Vec::new());
}

fn main() {
    let vertices = [[-1.0,-1.0], [-1.0, 1.0], [1.0, -1.0], [1.0, 1.0]];
    let indices = [0_u32, 1, 2, 3];

    let host = cpal::default_host();
    let device = host.default_output_device().expect("no default output device available");
    let stream = capture_output_audio(&device).unwrap();
    stream.play().unwrap();

    let default_window_size = [1000, 500];
    // taken from the glium documentation
    let mut event_loop = glium::glutin::event_loop::EventLoop::new();
    let wb = glium::glutin::window::WindowBuilder::new()
        .with_inner_size(glium::glutin::dpi::LogicalSize::new(default_window_size[0], default_window_size[1]))
        .with_title("Audio Spectrum");
    let cb = glium::glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    let vertex_buf = VertexBuffer::new(&display, &Vertex::vertices_from_array(&vertices)[..])
        .expect("Unable to create vertex buffer from given vertices");
    let index_buf = IndexBuffer::new(&display, glium::index::PrimitiveType::TriangleStrip, &indices).unwrap();
    let vertex_shader_src = include_str!("graphics/shaders/spectrum.vs").into();
    let fragment_shader_src = include_str!("graphics/shaders/spectrum.fs").into();

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let mut graphics = Spectrum::new(&display, 48_000, default_window_size[0], default_window_size[1], BUFFER_SIZE);
    graphics.compute_amplitudes();

    graphics.debug_print_amps();
    
    // let mut buffer = glium::uniforms::UniformBuffer::new(&display, SpectrumAmplitude { amps: [[0.0; 4]; 2048] }).unwrap();
    // {
    //     let mapping = buffer.map_read();
    //     println!("HAD: {:?}", &mapping.amps[0..5]);
    // }
    // let mut buf_2 = glium::uniforms::UniformBuffer::new(&display, SpectrumData { sample_rate: 48_000 }).unwrap();

    // let data = Vec::new();
    // let mut buf_3 = glium::uniforms::UniformBuffer::new(&display, data.)

    // let data = vec![[1_f32, 2_f32, 3_f32, 4_f32], [5.0, 6.0, 7.0, 8.0]];
    // let mut buffer = glium::buffer::Buffer::new(&display, &data[..], glium::buffer::BufferType::ShaderStorageBuffer, glium::buffer::BufferMode::Persistent).unwrap();

    // {
    //     let mapping = buffer.map_read();
    //     println!("HAD: {:?}", &*mapping);
    // }
    // let compute_shader = glium::program::ComputeShader::from_source(&display, include_str!("graphics/shaders/spectrum.comp")).unwrap();
    // compute_shader.execute(uniform! {SpectrumOut: &buffer}, 2, 1, 1);

    // {
    //     let mapping = buffer.map_read();
    //     println!("GOT: {:?}", &*mapping);
    // }



    
    event_loop.run(move |event, _, control_flow| {
        let next_frame_time = std::time::Instant::now() +
            std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                },
                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            glutin::event::Event::DeviceEvent { device_id: _, event } => {
                match event {
                    glutin::event::DeviceEvent::Key(ki) => {
                        match ki.scancode {
                            50 => display.gl_window().window().set_minimized(true),
                            _ => {},
                        }
                    },
                    _ => {},
                }
            }
            _ => return,
        }

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.draw(&vertex_buf, &index_buf, &program, &glium::uniforms::EmptyUniforms,
            &Default::default()).unwrap();
        target.finish().unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfft::num_complex::Complex;

    #[test]
    fn test_complex_to_float_array() {
        let a = vec![Complex::<f32>::new(1.0, 2.0), Complex::new(3.0, 4.0), Complex::new(-1.0, std::f32::consts::PI)];
        println!("init: {:?}", a);

        let (ptr, len, cap) = a.into_raw_parts();

        let b = unsafe {
            Vec::from_raw_parts(ptr as *mut [f32; 2], len, cap)
        };
        println!("final: {:?}", b);
    }
}
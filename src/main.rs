#![feature(vec_into_raw_parts)]
// uncomment for release
//#![windows_subsystem = "windows"]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate glium;

use audio_processing::fft::{FftCalculator, window::{BlackmanHarris, WindowFunction}, complex_to_vec4_arr};
use cpal::traits::{HostTrait, StreamTrait};
use rustfft::num_complex::ComplexFloat;
use system_audio::capture_output_audio;
use parking_lot::Mutex;
use glium::{Surface, VertexBuffer, IndexBuffer};
use graphics::vertex::Vertex;
use plotters::prelude::*;

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
//const BUFFER_SIZE: usize = 32768;
const BUFFER_SIZE: usize = 64;

lazy_static! {
    static ref AUDIO_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());
    static ref WAVE: Mutex<Vec<f32>> = Mutex::new(Vec::new());
}

fn draw_line_graph<T>(data: T)
where T: Into<Iterator<Item = (f32, f32)>>
{
    let root_area = BitMapBackend::new("images/2.6.png", (600, 400))
        .into_drawing_area();
    root_area.fill(&WHITE).unwrap();

    let mut ctx = ChartBuilder::on(&root_area)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .caption("Scatter Demo", ("sans-serif", 40))
        .build_cartesian_2d(-10..50, -10..50)
        .unwrap();

    ctx.configure_mesh().draw().unwrap();

    ctx.draw_series(data.into().iter().map(|point| Circle::new(*point, 5, &RED)))
        .unwrap();
}

fn main() {
    let vertices = [[-1.0,-1.0], [-1.0, 1.0], [1.0, -1.0], [1.0, 1.0]];
    let indices = [0_u32, 1, 2, 3];

    // obtain audio stream
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

    // build vertex and index buffers
    let vertex_buf = VertexBuffer::new(&display, &Vertex::vertices_from_array(&vertices)[..])
        .expect("Unable to create vertex buffer from given vertices");
    let index_buf = IndexBuffer::new(&display, glium::index::PrimitiveType::TriangleStrip, &indices).unwrap();
    
    // vertex and fragment shader values
    let vertex_shader_src = include_str!("graphics/shaders/spectrum.vs").into();
    let fragment_shader_src = include_str!("graphics/shaders/spectrum.fs").into();

    // build program from vertex and fragment shaders
    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    // build a spectrum object that will be used to run calculations using a compute shader
    // TODO: variable sample rate
    let mut graphics = Spectrum::new(&display, 48_000_f32, default_window_size[0], default_window_size[1], BUFFER_SIZE as u32);
    
    std::thread::sleep(std::time::Duration::from_secs(1));

    let mut fft_calc = FftCalculator::new(BUFFER_SIZE, 0).unwrap();
    let mut data = Vec::with_capacity(BUFFER_SIZE);
    for _ in 0..BUFFER_SIZE {
        data.push(rand::random::<f32>() * 2.0 - 1.0);
    }
    for i in 0..BUFFER_SIZE - 1 {
        data[i] = (data[i] + data[i + 1]) / 2.0;
    }
    for i in 0..BUFFER_SIZE - 1 {
        data[i] = (data[i] + data[i + 1]) / 2.0;
    }
    let comp = fft_calc.real_fft(&data[..], BlackmanHarris::real_window);

    let amps = comp.iter().map(|v| {
        v.abs()
    }).collect::<Vec<f32>>();
    println!("AMPS: {:?}", amps);
    
    
    let comp_buf = graphics.get_mut_comp_buf();
    println!("buf_size: {:?}", comp_buf.get_size());
    println!("vec_size: {:?}", std::mem::size_of_val(&comp[..]));
    comp_buf.write(&complex_to_vec4_arr(comp)[..]);

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
                _ => (),
            },
            _ => (),
        }

        graphics.compute_amplitudes();
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        let uniforms = uniform! {
            SpectrumIn: graphics.get_amp_buf(),
            resolution: [default_window_size[0], default_window_size[1]],
        };
        target.draw(&vertex_buf, &index_buf, &program, &uniforms,
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
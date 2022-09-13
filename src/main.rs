#![windows_subsystem = "windows"]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate glium;

use cpal::traits::{HostTrait, StreamTrait};
use system_audio::capture_output_audio;
use std::sync::Mutex;
use glium::{Surface, VertexBuffer, IndexBuffer};
use graphics::vertex::Vertex;

//use audio_graphics::waveform::Waveform;
//use audio_graphics::spectrum::Spectrum;
use ring_buffer::RingBuffer;

mod system_audio;
//mod audio_graphics;
mod ring_buffer;
mod graphics;

static mut SAMPLE_RATE: f32 = 48000_f32;
const BUFFER_SIZE: usize = 32768;

lazy_static! {
    static ref AUDIO_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());
    static ref WAVE: Mutex<Vec<f32>> = Mutex::new(Vec::new());
}

fn main() {

    let vertices = [[0.2, 0.1], [0.1, 0.5], [0.2, 0.9], [0.8, 0.9], [0.9, 0.5], [0.8, 0.1]];
    let indices: [u32; 12] = [0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5];

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
    let index_buf = IndexBuffer::new(&display, glium::index::PrimitiveType::TrianglesList, &indices).unwrap();
    let vertex_shader_src = r#"
        #version 140
        in vec2 pos;

        void main() {
            gl_Position = vec4(pos, 0.0, 1.0);
        }
    "#;
    let fragment_shader_src = r#"
        #version 140
        out vec4 color;

        void main() {
            color = vec4(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();
    
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
        /*target.draw(&vertex_buffer, &indices, &program, &glium::uniforms::EmptyUniforms,
                    &Default::default()).unwrap();*/
        target.draw(&vertex_buf, &index_buf, &program, &glium::uniforms::EmptyUniforms,
            &Default::default()).unwrap();
        target.finish().unwrap();
    });
    //let opengl = OpenGL::V3_2;
    /*let mut window: Window = WindowSettings::new("Audio Spectrum", window_size)
        .samples(4)
        .graphics_api(opengl)
        .exit_on_esc(true)
        .decorated(true)
        .transparent(true)
        .build()
        .unwrap();*/

    // uncomment this and comment out the graph definition below to get a waveform visualizer
    /*let mut graph = Waveform {
        gl: GlGraphics::new(opengl),
        radius: 0.5_f64,
    };*/
    /*let mut graph = Spectrum::new(GlGraphics::new(opengl), [0.619, 0.733, 1.0, 1.0], 1_f64, 0.015, [1000_f64, 500_f64], 1_f32);
    //let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            graph.render(&args);
        }

        if let Some(_args) = e.update_args() {}
    }*/
}
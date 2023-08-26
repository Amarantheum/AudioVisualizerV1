#[macro_use]
extern crate lazy_static;

use cpal::traits::{HostTrait, StreamTrait};
use system_audio::capture_output_audio;
use std::sync::Arc;
use egui::mutex::Mutex;
use eframe::{egui_glow::{glow, self}, glow::{NativeShader, HasContext}};


//use audio_graphics::waveform::Waveform;
//use audio_graphics::spectrum::Spectrum;
use ring_buffer::RingBuffer;

mod system_audio;
//mod audio_graphics;
mod ring_buffer;

static mut SAMPLE_RATE: f32 = 48000_f32;
const BUFFER_SIZE: usize = 32768;

lazy_static! {
    static ref AUDIO_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());
    static ref WAVE: Mutex<Vec<f32>> = Mutex::new(Vec::new());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("My egui App", native_options, Box::new(|cc| Box::new(AudioAnalyzerApp::new(cc))))?;
    Ok(())
}

struct AudioAnalyzerApp {
    audio_host: cpal::Host,
    audio_device: cpal::Device,
    audio_stream: cpal::Stream,

    rotating_triangle: Arc<Mutex<WaveformGraphics>>,
    angle: f32,
}

impl AudioAnalyzerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        // set up audio stream
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no default output device available");
        let stream = capture_output_audio(&device).unwrap();
        stream.play().unwrap();

        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        Self {
            audio_host: host,
            audio_device: device,
            audio_stream: stream,

            rotating_triangle: Arc::new(Mutex::new(WaveformGraphics::new(gl))),
            angle: 0.0,
        }
    }
}

impl eframe::App for AudioAnalyzerApp {
   fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("The triangle is being painted using ");
                ui.hyperlink_to("glow", "https://github.com/grovesNL/glow");
                ui.label(" (OpenGL).");
            });

            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.custom_painting(ui);
            });
            ui.label("Drag to rotate!");
        });

       ctx.request_repaint();
   }
}

impl AudioAnalyzerApp {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let height = ui.available_height() / 2.0;
        let (rect, response) = {
            ui.allocate_exact_size(egui::Vec2 { x: width, y: height }, egui::Sense::click())
        };

        // Clone locals so we can move them into the paint callback:
        let rotating_triangle = self.rotating_triangle.clone();

        let callback = egui::PaintCallback {
            rect,
            callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                rotating_triangle.lock().paint(painter.gl(), width as u32);
            })),
        };
        ui.painter().add(callback);
    }
}

struct WaveformGraphics {
    signal_buf: glow::Buffer,
    bound_buf: glow::Buffer,
    signal_length_loc: glow::UniformLocation,
    line_width_loc: glow::UniformLocation,
    compute_program: glow::Program,
    program: glow::Program,
    vertex_array: glow::VertexArray,
    num_pixels_loc: glow::UniformLocation,
}

impl WaveformGraphics {
    fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        let compute_program = unsafe {
            let compute_program = gl.create_program().expect("Cannot create program");

            let shader_source = include_str!("./shaders/waveform.comp");
            let shader = gl.create_shader(glow::COMPUTE_SHADER).expect("Cannot create shader");
            gl.shader_source(shader, shader_source);
            // compile shader
            gl.compile_shader(shader);
            // check status of compile
            if !gl.get_shader_compile_status(shader) {
                panic!("Failed to compile compute shader: {}", gl.get_shader_info_log(shader));
            }
            gl.attach_shader(compute_program, shader);
            // link program
            gl.link_program(compute_program);
            // check status of link
            if !gl.get_program_link_status(compute_program) {
                panic!("Failed to link compute program: {}", gl.get_program_info_log(compute_program));
            }
            gl.detach_shader(compute_program, shader);
            gl.delete_shader(shader);

            compute_program
        };

        let signal_buf = unsafe { gl.create_buffer().expect("failed to create wf_signal_buf") };
        let bound_buf = unsafe { gl.create_buffer().expect("failed to create wf_bound_buf") };
        
        // TODO: algorithms to decrease the gpu memory footprint
        unsafe {
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(signal_buf));
            // DYNAMIC_DRAW because we are reusing the buffer and writing from app to GL but not writing
            // size of 65536 because the plan is to not support more than 65536 samples for the size of the audio buffer
            gl.buffer_data_u8_slice(glow::SHADER_STORAGE_BUFFER, &[0; 2_usize.pow(16) * core::mem::size_of::<f32>()], glow::DYNAMIC_DRAW);
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(bound_buf));
            // DYNAMIC_COPY because we are reusing the buffer and everything else occurs in GL
            // size of 32000 because the plan is to not support more than 16000 pixels for the size of the window (16000 * 2 (upper and lower bound) = 32000)
            gl.buffer_data_u8_slice(glow::SHADER_STORAGE_BUFFER, &[0; 32000 * core::mem::size_of::<f32>()], glow::DYNAMIC_COPY);
        }

        let signal_length_loc = unsafe {
            gl.get_uniform_location(compute_program, "u_signal_length")
                .expect("Cannot get uniform location")
        };

        let line_width_loc = unsafe {
            gl.get_uniform_location(compute_program, "u_line_width")
                .expect("Cannot get uniform location")
        };

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let shader_sources = [
                (glow::VERTEX_SHADER, include_str!("./shaders/waveform.vs"),),
                (glow::FRAGMENT_SHADER, include_str!("./shaders/waveform.fs")),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &shader_source);
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Failed to compile {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "{}",
                gl.get_program_info_log(program)
            );

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            let num_pixels_loc = unsafe {
                gl.get_uniform_location(program, "u_num_pixels")
                    .expect("Cannot get uniform location")
            };

            Self {
                signal_buf,
                bound_buf,
                signal_length_loc,
                line_width_loc,
                compute_program,
                program,
                vertex_array,
                num_pixels_loc,
            }
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.compute_program);
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&self, gl: &glow::Context, width: u32) {
        use glow::HasContext as _;
        unsafe {
            let data_rb = AUDIO_BUFFER.lock();
            let data = data_rb.get_raw();
            let data_u8: &[u8] = core::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * core::mem::size_of::<f32>(),
            );

            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.signal_buf));
            gl.buffer_sub_data_u8_slice(glow::SHADER_STORAGE_BUFFER, 0, &data_u8);
            gl.use_program(Some(self.compute_program));
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(self.signal_buf));
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(self.bound_buf));
            gl.uniform_1_u32(Some(&self.signal_length_loc), data.len() as u32);
            gl.uniform_1_f32(Some(&self.line_width_loc), 0.01_f32);
            gl.dispatch_compute(width, 1, 1);
            gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
            std::mem::drop(data_rb);

            // let mut dst_data: Vec<f32> = vec![0_f32; 30_000];
            // let dst_data_u8: &mut [u8] = core::slice::from_raw_parts_mut(
            //     dst_data.as_mut_ptr() as *mut u8,
            //     dst_data.len() * core::mem::size_of::<f32>(),
            // );
            // gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.bound_buf));
            // gl.get_buffer_sub_data(glow::SHADER_STORAGE_BUFFER, 0, dst_data_u8);

            //println!("dst_data: {:?}", &dst_data[0..20]);

            gl.use_program(Some(self.program));
            gl.uniform_1_u32(Some(&self.num_pixels_loc), width);
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(self.bound_buf));
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
}
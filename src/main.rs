#[macro_use]
extern crate lazy_static;

use cpal::traits::{HostTrait, StreamTrait};
use parking_lot::Mutex;

use ring_buffer::RingBuffer;
use system_audio::capture_output_audio;

mod ring_buffer;
mod spectrum_renderer;
mod system_audio;
mod wgpu_renderer;

use spectrum_renderer::SpectrumRenderer;
use wgpu_renderer::WaveformRenderer;

pub static mut SAMPLE_RATE: f32 = 48000_f32;
pub const BUFFER_SIZE: usize = 32768;

#[derive(Clone, PartialEq)]
pub enum WaveformMode {
    Scroll,  // Data scrolls from right to left
    Sweep,   // New data overwrites old, sweep line moves across
}

#[derive(Clone)]
pub struct SpectrumSettings {
    pub fft_size: usize,
    pub falloff_speed: f32, // dB per second
    pub vertical_offset: f32, // 0-1 range to shift spectrum up
}

#[derive(Clone)]
pub struct WaveformSettings {
    pub mode: WaveformMode,
}

impl Default for SpectrumSettings {
    fn default() -> Self {
        Self {
            fft_size: 8192,
            falloff_speed: 2.0, // 2 dB/s decay
            vertical_offset: 0.5,
        }
    }
}

impl Default for WaveformSettings {
    fn default() -> Self {
        Self {
            mode: WaveformMode::Scroll,
        }
    }
}

lazy_static! {
    pub static ref AUDIO_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Audio Visualizer"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Audio Visualizer",
        options,
        Box::new(|cc| Box::new(AudioVisualizerApp::new(cc))),
    )
}

struct AudioVisualizerApp {
    _audio_stream: cpal::Stream,
    waveform_renderer: Option<WaveformRenderer>,
    spectrum_renderer: Option<SpectrumRenderer>,
    wgpu_render_state: Option<eframe::egui_wgpu::RenderState>,
    spectrum_settings: SpectrumSettings,
    waveform_settings: WaveformSettings,
    last_frame_time: std::time::Instant,
    show_settings: bool,
}

impl AudioVisualizerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set up audio stream
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No default output device available");
        let stream = capture_output_audio(&device).expect("Failed to capture audio");
        stream.play().expect("Failed to play stream");

        let spectrum_settings = SpectrumSettings::default();

        // Initialize wgpu renderers
        let (waveform_renderer, spectrum_renderer, wgpu_render_state) = 
            if let Some(render_state) = cc.wgpu_render_state.clone() {
                let device = &render_state.device;
                (
                    Some(WaveformRenderer::new(device)),
                    Some(SpectrumRenderer::new(device, spectrum_settings.fft_size)),
                    Some(render_state),
                )
            } else {
                (None, None, None)
            };

        Self {
            _audio_stream: stream,
            waveform_renderer,
            spectrum_renderer,
            wgpu_render_state,
            spectrum_settings,
            waveform_settings: WaveformSettings::default(),
            last_frame_time: std::time::Instant::now(),
            show_settings: false,
        }
    }

    fn draw_waveform(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        let width = available_size.x.max(1.0) as u32;
        let height = available_size.y.max(1.0) as u32;
        let sweep_mode = self.waveform_settings.mode == WaveformMode::Sweep;

        if let (Some(renderer), Some(render_state)) = 
            (&mut self.waveform_renderer, &self.wgpu_render_state) 
        {
            // Render to texture
            if let Some(pixels) = renderer.render_to_pixels(&render_state.device, &render_state.queue, width, height, sweep_mode) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    &pixels,
                );
                let texture = ui.ctx().load_texture(
                    "waveform_texture",
                    image,
                    egui::TextureOptions::LINEAR,
                );
                ui.image(&texture);
            }
        }
    }

    fn draw_spectrum(&mut self, ui: &mut egui::Ui, dt: f32) {
        let available_size = ui.available_size();
        let width = available_size.x.max(1.0) as u32;
        let height = available_size.y.max(1.0) as u32;

        if let (Some(renderer), Some(render_state)) = 
            (&mut self.spectrum_renderer, &self.wgpu_render_state) 
        {
            // Render to texture with falloff
            if let Some(pixels) = renderer.render_to_pixels(
                &render_state.device, 
                &render_state.queue, 
                width, 
                height,
                self.spectrum_settings.falloff_speed,
                dt,
                self.spectrum_settings.vertical_offset,
            ) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    &pixels,
                );
                let texture = ui.ctx().load_texture(
                    "spectrum_texture",
                    image,
                    egui::TextureOptions::LINEAR,
                );
                ui.image(&texture);
            }
        }
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("FFT Size:");
            egui::ComboBox::from_id_source("fft_size")
                .selected_text(format!("{}", self.spectrum_settings.fft_size))
                .show_ui(ui, |ui| {
                    for &size in &[1024, 2048, 4096, 8192, 16384, 32768] {
                        if ui.selectable_value(&mut self.spectrum_settings.fft_size, size, format!("{}", size)).changed() {
                            // Recreate spectrum renderer with new FFT size
                            if let Some(render_state) = &self.wgpu_render_state {
                                self.spectrum_renderer = Some(SpectrumRenderer::new(&render_state.device, size));
                            }
                        }
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Falloff (dB/s):");
            ui.add(egui::Slider::new(&mut self.spectrum_settings.falloff_speed, 0.0..=10.0));
        });

        ui.horizontal(|ui| {
            ui.label("Vertical Offset:");
            ui.add(egui::Slider::new(&mut self.spectrum_settings.vertical_offset, 0.0..=1.0));
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Waveform Mode:");
            ui.selectable_value(&mut self.waveform_settings.mode, WaveformMode::Scroll, "Scroll");
            ui.selectable_value(&mut self.waveform_settings.mode, WaveformMode::Sweep, "Sweep");
        });
    }
}

impl eframe::App for AudioVisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate delta time
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        // Settings panel
        egui::TopBottomPanel::top("settings_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.show_settings, "⚙ Settings");
                if self.show_settings {
                    ui.separator();
                    self.draw_settings(ui);
                }
            });
        });

        // Top panel for waveform
        egui::TopBottomPanel::top("waveform_panel")
            .resizable(true)
            .default_height(ctx.screen_rect().height() / 2.0)
            .min_height(100.0)
            .show(ctx, |ui| {
                ui.heading("Waveform");
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.draw_waveform(ui);
                });
            });

        // Bottom panel for spectrum
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Spectrum Analyzer");
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.draw_spectrum(ui, dt);
            });
        });

        // Request repaint at ~60fps instead of as fast as possible
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

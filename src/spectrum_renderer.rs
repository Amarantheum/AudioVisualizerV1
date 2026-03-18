use bytemuck::{Pod, Zeroable};
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

use crate::{AUDIO_BUFFER, BUFFER_SIZE, SAMPLE_RATE};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpectrumComputeUniforms {
    sample_rate: f32,
    scale: f32,
    buf_size: u32,
    width: u32,
    log_min_freq: f32,
    log_freq_range: f32,
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpectrumRenderUniforms {
    width: u32,
    bar_width: f32,
    min_db: f32,
    max_db: f32,
    vertical_offset: f32,
    height: f32,
    _padding2: f32,
    _padding3: f32,
}

pub struct SpectrumRenderer {
    #[allow(dead_code)]
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    fft_buffer: wgpu::Buffer,
    amplitudes_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    compute_uniform_buffer: wgpu::Buffer,
    render_uniform_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,
    output_buffer: Option<wgpu::Buffer>,
    texture_size: (u32, u32),
    fft: Arc<dyn rustfft::Fft<f32>>,
    fft_size: usize,
    // Smoothed amplitude state for falloff
    smoothed_amplitudes: Vec<f32>,
}

impl SpectrumRenderer {
    pub fn new(device: &wgpu::Device, fft_size: usize) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Spectrum Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/spectrum.wgsl").into()),
        });

        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Spectrum Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/spectrum_render.wgsl").into()),
        });

        // FFT output buffer (complex numbers)
        let fft_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FFT Buffer"),
            size: (BUFFER_SIZE * 2 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let amplitudes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Amplitudes Buffer"),
            size: (16000 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spectrum Compute Uniform Buffer"),
            size: std::mem::size_of::<SpectrumComputeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let render_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spectrum Render Uniform Buffer"),
            size: std::mem::size_of::<SpectrumRenderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Spectrum Compute Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Spectrum Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Spectrum Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Spectrum Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Spectrum Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Spectrum Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Spectrum Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: fft_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: amplitudes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: compute_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Spectrum Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: amplitudes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let fft = FftPlanner::new().plan_fft_forward(fft_size);

        Self {
            compute_pipeline,
            render_pipeline,
            fft_buffer,
            amplitudes_buffer,
            compute_uniform_buffer,
            render_uniform_buffer,
            compute_bind_group,
            render_bind_group,
            texture: None,
            texture_view: None,
            output_buffer: None,
            texture_size: (0, 0),
            fft,
            fft_size,
            smoothed_amplitudes: Vec::new(),
        }
    }

    fn ensure_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.texture_size != (width, height) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Spectrum Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            self.texture_view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
            self.texture = Some(texture);
            
            // Create output buffer for reading pixels
            let bytes_per_row = (width * 4 + 255) & !255;
            let buffer_size = (bytes_per_row * height) as u64;
            self.output_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Spectrum Output Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
            
            self.texture_size = (width, height);
        }
    }

    fn blackman_harris(buffer: &mut [f32]) {
        use std::f32::consts::PI;
        let n = buffer.len() as f32;
        for (i, sample) in buffer.iter_mut().enumerate() {
            let window = 0.35875
                - 0.48829 * ((2.0 * i as f32) * PI / (n - 1.0)).cos()
                + 0.14128 * ((4.0 * i as f32) * PI / (n - 1.0)).cos()
                - 0.01168 * ((6.0 * i as f32) * PI / (n - 1.0)).cos();
            *sample *= window;
        }
    }

    pub fn render_to_pixels(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        falloff_speed: f32,
        dt: f32,
        vertical_offset: f32,
    ) -> Option<Vec<u8>> {
        if width == 0 || height == 0 {
            return None;
        }

        self.ensure_texture(device, width, height);

        // Get most recent audio data for FFT
        let audio_data = AUDIO_BUFFER.lock().get_recent(self.fft_size);
        
        // Apply window function
        let mut windowed = audio_data;
        Self::blackman_harris(&mut windowed);
        
        // Convert to complex
        let mut complex_buffer: Vec<Complex<f32>> = windowed
            .iter()
            .map(|&x| Complex { re: x, im: 0.0 })
            .collect();
        complex_buffer.resize(self.fft_size, Complex { re: 0.0, im: 0.0 });
        
        // Perform FFT
        self.fft.process(&mut complex_buffer);
        
        // Calculate magnitudes with falloff
        let scale = 2.0 / self.fft_size as f32;
        let num_bins = self.fft_size / 2;
        
        // Ensure smoothed_amplitudes is the right size
        if self.smoothed_amplitudes.len() != num_bins {
            self.smoothed_amplitudes = vec![-100.0; num_bins];
        }
        
        // Calculate current magnitudes and apply falloff
        let falloff_amount = falloff_speed * dt;
        for i in 0..num_bins {
            let c = complex_buffer[i];
            let mag = (c.re * c.re + c.im * c.im).sqrt() * scale;
            let db = if mag < 0.00001 { -100.0 } else { mag.log10() };
            
            // Apply falloff: new value is max of current or decayed previous
            if db > self.smoothed_amplitudes[i] {
                self.smoothed_amplitudes[i] = db;
            } else {
                self.smoothed_amplitudes[i] -= falloff_amount;
                self.smoothed_amplitudes[i] = self.smoothed_amplitudes[i].max(-100.0);
            }
        }
        
        // Map smoothed amplitudes to pixel columns using log frequency scale
        let sample_rate = unsafe { SAMPLE_RATE };
        let log_min_freq = 20.0_f32.log10();
        let log_freq_range = 20000.0_f32.log10() - log_min_freq;
        
        let mut pixel_amplitudes: Vec<f32> = Vec::with_capacity(width as usize);
        for pixel in 0..width {
            let t = pixel as f32 / width as f32;
            let freq = 10.0_f32.powf(log_min_freq + t * log_freq_range);
            let bin = freq * self.fft_size as f32 / sample_rate;
            let bin_idx = (bin as usize).min(num_bins - 1);
            pixel_amplitudes.push(self.smoothed_amplitudes[bin_idx]);
        }
        
        // Upload smoothed amplitudes directly to GPU
        let amp_bytes: &[u8] = bytemuck::cast_slice(&pixel_amplitudes);
        queue.write_buffer(&self.amplitudes_buffer, 0, amp_bytes);

        // Update render uniforms
        let render_uniforms = SpectrumRenderUniforms {
            width,
            bar_width: 1.0,
            min_db: -3.0,
            max_db: 1.0,
            vertical_offset,
            height: height as f32,
            _padding2: 0.0,
            _padding3: 0.0,
        };
        queue.write_buffer(&self.render_uniform_buffer, 0, bytemuck::bytes_of(&render_uniforms));

        let bytes_per_row = (width * 4 + 255) & !255;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Spectrum Encoder"),
        });

        // Render pass (skip compute pass - we upload amplitudes directly)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Spectrum Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.texture_view.as_ref().unwrap(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }

        // Copy texture to buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: self.texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: self.output_buffer.as_ref().unwrap(),
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Read pixels from buffer
        let buffer_slice = self.output_buffer.as_ref().unwrap().slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        
        if rx.recv().unwrap().is_ok() {
            let data = buffer_slice.get_mapped_range();
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
            
            // Copy data row by row (handle padding)
            for y in 0..height {
                let start = (y * bytes_per_row) as usize;
                let end = start + (width * 4) as usize;
                pixels.extend_from_slice(&data[start..end]);
            }
            
            drop(data);
            self.output_buffer.as_ref().unwrap().unmap();
            
            Some(pixels)
        } else {
            None
        }
    }
}

use bytemuck::{Pod, Zeroable};

use crate::{AUDIO_BUFFER, BUFFER_SIZE};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct WaveformComputeUniforms {
    signal_length: u32,
    width: u32,
    _padding: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct WaveformRenderUniforms {
    width: u32,
    line_width: f32,
    height: f32,
    _padding: f32,
}

pub struct WaveformRenderer {
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    signal_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    bounds_buffer: wgpu::Buffer,
    compute_uniform_buffer: wgpu::Buffer,
    render_uniform_buffer: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,
    output_buffer: Option<wgpu::Buffer>,
    texture_size: (u32, u32),
}

impl WaveformRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        // Create compute shader module
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Waveform Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/waveform.wgsl").into()),
        });

        // Create render shader module
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Waveform Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/waveform_render.wgsl").into()),
        });

        // Create buffers
        let signal_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Signal Buffer"),
            size: (BUFFER_SIZE * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bounds_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bounds Buffer"),
            size: (16000 * 2 * std::mem::size_of::<f32>()) as u64, // max 16000 pixels, 2 floats per pixel
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let compute_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Uniform Buffer"),
            size: std::mem::size_of::<WaveformComputeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let render_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Render Uniform Buffer"),
            size: std::mem::size_of::<WaveformRenderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create compute bind group layout
        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
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

        // Create render bind group layout
        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
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

        // Create compute pipeline
        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Waveform Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Waveform Render Pipeline"),
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

        // Create bind groups
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: signal_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bounds_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: compute_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bounds_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            compute_pipeline,
            render_pipeline,
            signal_buffer,
            bounds_buffer,
            compute_uniform_buffer,
            render_uniform_buffer,
            compute_bind_group,
            render_bind_group,
            texture: None,
            texture_view: None,
            output_buffer: None,
            texture_size: (0, 0),
        }
    }

    fn ensure_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.texture_size != (width, height) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Waveform Texture"),
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
            let bytes_per_row = (width * 4 + 255) & !255; // Align to 256 bytes
            let buffer_size = (bytes_per_row * height) as u64;
            self.output_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Waveform Output Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
            
            self.texture_size = (width, height);
        }
    }

    pub fn render_to_pixels(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        sweep_mode: bool,
    ) -> Option<Vec<u8>> {
        if width == 0 || height == 0 {
            return None;
        }

        self.ensure_texture(device, width, height);

        // Get audio data
        // In scroll mode: data is arranged chronologically (oldest to newest left to right)
        // In sweep mode: data is in raw buffer order, so the write position creates a sweep effect
        let audio_data = if sweep_mode {
            // Raw buffer order - the write position naturally sweeps across
            let (data, _write_pos) = AUDIO_BUFFER.lock().get_raw_with_position();
            data
        } else {
            // Chronological order - oldest on left, newest on right
            AUDIO_BUFFER.lock().get_vec()
        };
        
        let audio_bytes: &[u8] = bytemuck::cast_slice(&audio_data);
        queue.write_buffer(&self.signal_buffer, 0, audio_bytes);

        // Update compute uniforms
        let compute_uniforms = WaveformComputeUniforms {
            signal_length: audio_data.len() as u32,
            width,
            _padding: [0; 2],
        };
        queue.write_buffer(&self.compute_uniform_buffer, 0, bytemuck::bytes_of(&compute_uniforms));

        // Update render uniforms
        let render_uniforms = WaveformRenderUniforms {
            width,
            line_width: 0.01,
            height: height as f32,
            _padding: 0.0,
        };
        queue.write_buffer(&self.render_uniform_buffer, 0, bytemuck::bytes_of(&render_uniforms));

        let bytes_per_row = (width * 4 + 255) & !255;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Waveform Encoder"),
        });

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Waveform Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch_workgroups((width + 63) / 64, 1, 1);
        }

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Waveform Render Pass"),
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

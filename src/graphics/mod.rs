use glium::backend::Facade;
use glium::buffer::Buffer;
use glium::program::ComputeShader;

pub mod vertex;

pub struct Spectrum {
    pub sample_rate: u32,
    amp_buffer: Buffer<[[f32; 4]]>,
    // read as [Re(z_0), Im(z_0), Re(z_1), Im(z_1), ...]
    complex_buf: Buffer<[[f32; 4]]>,
    compute_shader: ComputeShader,
    width: u32,
    height: u32,
    buf_size: usize,
    scale: f32,
    start_freq: u32,
    end_freq: u32,
}

impl Spectrum {
    /// Create a new spectrum object for generating the graphics
    pub fn new<F>(facade: &F, sample_rate: u32, width: u32, height: u32, buf_size: usize) -> Self
    where F: Facade
    {
        assert!(buf_size.is_power_of_two());
        Self {
            sample_rate,
            compute_shader: glium::program::ComputeShader::from_source(facade, include_str!("shaders/spectrum.comp")).unwrap(),
            width,
            height,
            buf_size,
            amp_buffer: Self::gen_amp_buf(facade, width),
            complex_buf: Self::gen_comp_buf(facade, buf_size),
            scale: 1.0,
            start_freq: 20,
            end_freq: 20_000,
        }
    }

    #[inline]
    pub fn compute_amplitudes(&self) {
        let uniforms = uniform! {
            SpectrumOut: &self.amp_buffer,
            ComplexIn: &self.complex_buf,
            sample_rate: self.sample_rate,
            start_freq: self.start_freq,
            end_freq: self.end_freq,
        };
        self.compute_shader.execute(uniforms, self.width + 1, 1, 1)
    }
}

// extra helpers
impl Spectrum {
    #[inline]
    fn gen_amp_buf<F>(facade: &F, width: u32) -> Buffer<[[f32; 4]]>
    where F: Facade
    {
        let amps = vec![[0_f32; 4]; (width as usize) / 4 + 1];
        Buffer::new(facade, &amps[..], glium::buffer::BufferType::ShaderStorageBuffer, glium::buffer::BufferMode::Persistent).unwrap()
    }

    #[inline]
    fn gen_comp_buf<F>(facade: &F, buf_size: usize) -> Buffer<[[f32; 4]]>
    where F: Facade
    {
        // divide by 2 instead of 4 because each value has Re and Im component
        let complex = vec![[0_f32; 4]; buf_size / 2];
        Buffer::new(facade, &complex[..], glium::buffer::BufferType::UniformBuffer, glium::buffer::BufferMode::Persistent).unwrap()
    }

    #[inline]
    pub fn resize_facade<F>(&mut self, facade: &F, width: u32, height: u32)
    where F: Facade
    {
        self.width = width;
        self.height = height;
        self.amp_buffer = Self::gen_amp_buf(facade, width)
    }

    #[inline]
    pub fn resize_buf<F>(&mut self, facade: &F, buf_size: usize)
    where F: Facade
    {
        assert!(buf_size.is_power_of_two());
        self.buf_size = buf_size;
        self.complex_buf = Self::gen_comp_buf(facade, buf_size)
    }

    #[inline]
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    #[inline]
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    #[inline]
    pub fn set_start_freq(&mut self, start_freq: u32) {
        self.start_freq = start_freq
    }

    #[inline]
    pub fn set_end_freq(&mut self, end_freq: u32) {
        self.end_freq = end_freq
    }

    pub fn debug_print_amps(&mut self) {
        println!("{:?}", &*self.amp_buffer.map())
    }
}
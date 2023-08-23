use glium::backend::Facade;
use glium::buffer::Buffer;
use glium::program::ComputeShader;

pub mod vertex;

pub struct Spectrum {
    // the sample rate in terms of how much zero padding there is (i.e. doubling the length with zero padding as sample rate of 48_000 => 96_000)
    sample_rate: f32,
    amp_buffer: Buffer<[[f32; 4]]>,
    // read as [Re(z_0), Im(z_0), Re(z_1), Im(z_1), ...]
    complex_buf: Buffer<[[f32; 4]]>,
    freq_buf: Buffer<[[f32; 4]]>,
    compute_shader: ComputeShader,
    width: u32,
    height: u32,
    buf_size: u32,
    scale: f32,
    start_freq: f32,
    end_freq: f32,
}

impl Spectrum
{
    /// Create a new spectrum object for generating the graphics
    pub fn new<F: Facade>(facade: &F, sample_rate: f32, width: u32, height: u32, buf_size: u32) -> Self {
        assert!(buf_size.is_power_of_two());
        Self {
            sample_rate,
            compute_shader: glium::program::ComputeShader::from_source(facade, include_str!("shaders/spectrum.comp")).unwrap(),
            width,
            height,
            buf_size,
            amp_buffer: Self::gen_amp_buf(facade, width),
            complex_buf: Self::gen_comp_buf(facade, buf_size),
            freq_buf: Self::gen_freq_buf(facade, 20_f32, 20_000_f32, sample_rate, width, buf_size),
            scale: 1.0,
            start_freq: 20_f32,
            end_freq: 20_000_f32,
        }
    }

    #[inline]
    pub fn compute_amplitudes(&self) {
        let uniforms = uniform! {
            SpectrumOut: &self.amp_buffer,
            ComplexIn: &self.complex_buf,
            PixFreqs: &self.freq_buf,
            sample_rate: self.sample_rate,
            scale: self.scale,
            buf_size: self.buf_size,
            width: self.width,
        };
        self.compute_shader.execute(uniforms, self.width, 1, 1)
    }

    // TODO
    #[inline]
    pub fn resize(&mut self) {

    }
}

// extra helpers for Spectrum
impl Spectrum {
    // return a SSBO
    #[inline]
    fn gen_amp_buf<F: Facade>(facade: &F, width: u32) -> Buffer<[[f32; 4]]> {
        let amps = vec![[0_f32; 4]; (width as usize) / 4 + 1];
        Buffer::new(facade, &amps[..], glium::buffer::BufferType::ShaderStorageBuffer, glium::buffer::BufferMode::Persistent).unwrap()
    }

    #[inline]
    fn gen_comp_buf<F: Facade>(facade: &F, buf_size: u32) -> Buffer<[[f32; 4]]> {
        println!("buf_size: {}", buf_size);
        // divide by 2 instead of 4 because each value has Re and Im component
        let complex = vec![[0_f32; 4]; buf_size as usize / 2];
        println!("comp_size: {:?}", std::mem::size_of_val(&complex[..]));
        let b = Buffer::new(facade, &complex[..], glium::buffer::BufferType::UniformBuffer, glium::buffer::BufferMode::Persistent).unwrap();
        println!("b_size: {:?}", b.get_size());
        b
    }

    #[inline]
    fn gen_freq_buf<F: Facade>(facade: &F, start_freq: f32, end_freq: f32, sample_rate: f32, width: u32, buf_size: u32) -> Buffer<[[f32; 4]]> {
        let mut values = Vec::with_capacity(width as usize / 4 + 1);
        for i in 0..=(width / 4) {
            let mut v = [0.0; 4];
            for j in 0..4 {
                v[j] = (end_freq / start_freq).powf((i as usize * 4 + j) as f32 / width as f32) * start_freq / sample_rate * buf_size as f32;
            }
            values.push(v);
        }
        Buffer::new(facade, &values[..], glium::buffer::BufferType::UniformBuffer, glium::buffer::BufferMode::Immutable).unwrap()
    }

    #[inline]
    pub fn resize_facade<F: Facade>(&mut self, facade: &F, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.amp_buffer = Self::gen_amp_buf(facade, width);
        self.self_generate_freq_buf(facade);
    }

    #[inline]
    pub fn resize_buf<F: Facade>(&mut self, facade: &F, buf_size: u32) {
        assert!(buf_size.is_power_of_two());
        self.buf_size = buf_size;
        self.complex_buf = Self::gen_comp_buf(facade, buf_size);
        self.self_generate_freq_buf(facade);
    }

    #[inline]
    fn self_generate_freq_buf<F: Facade>(&mut self, facade: &F) {
        self.freq_buf = Self::gen_freq_buf(facade, self.start_freq, self.end_freq, self.sample_rate, self.width, self.buf_size);
    }

    #[inline]
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    #[inline]
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    #[inline]
    pub fn set_start_freq<F: Facade>(&mut self, facade: &F, start_freq: f32) {
        self.start_freq = start_freq;
        self.self_generate_freq_buf(facade);
    }

    #[inline]
    pub fn set_end_freq<F: Facade>(&mut self, facade: &F, end_freq: f32) {
        self.end_freq = end_freq;
        self.self_generate_freq_buf(facade);
    }

    #[inline]
    pub fn get_amp_buf(&self) -> &Buffer<[[f32; 4]]> {
        &self.amp_buffer
    }

    #[inline]
    pub fn get_mut_comp_buf(&mut self) -> &mut Buffer<[[f32; 4]]> {
        &mut self.complex_buf
    }

    pub fn debug_print_amps(&mut self) {
        println!("{:?}", &*self.amp_buffer.map())
    }
}
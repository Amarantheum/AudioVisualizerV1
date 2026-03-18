// Spectrum compute shader - calculates FFT magnitude for each pixel column

@group(0) @binding(0) var<storage, read> fft_data: array<vec2<f32>>; // Complex numbers (re, im)
@group(0) @binding(1) var<storage, read_write> amplitudes: array<f32>;

struct SpectrumUniforms {
    sample_rate: f32,
    scale: f32,
    buf_size: u32,
    width: u32,
    log_min_freq: f32,
    log_freq_range: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(2) var<uniform> uniforms: SpectrumUniforms;

fn get_magnitude(index: u32) -> f32 {
    let c = fft_data[index];
    return sqrt(c.x * c.x + c.y * c.y) * uniforms.scale;
}

fn log_magnitude(index: u32) -> f32 {
    let mag = get_magnitude(index);
    if (mag < 0.00001) {
        return -5.0; // Floor value
    }
    return log(mag) / log(10.0);
}

fn pixel_to_freq(pixel: u32) -> f32 {
    // Logarithmic frequency mapping: 20Hz to 20kHz
    let t = f32(pixel) / f32(uniforms.width);
    return pow(10.0, uniforms.log_min_freq + t * uniforms.log_freq_range);
}

fn freq_to_bin(freq: f32) -> f32 {
    return freq * f32(uniforms.buf_size) / uniforms.sample_rate;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel = global_id.x;
    if (pixel >= uniforms.width) {
        return;
    }
    
    let freq = pixel_to_freq(pixel);
    let next_freq = pixel_to_freq(pixel + 1u);
    let bin = freq_to_bin(freq);
    let next_bin = freq_to_bin(next_freq);
    
    // If current pixel encompasses multiple bins, find max
    if (next_bin - bin > 1.0) {
        var max_mag = log_magnitude(u32(bin));
        for (var i = u32(bin) + 1u; i < u32(next_bin); i = i + 1u) {
            max_mag = max(max_mag, log_magnitude(i));
        }
        amplitudes[pixel] = max_mag;
    } else {
        // Interpolate between bins
        let y0 = log_magnitude(u32(bin));
        let y1 = log_magnitude(u32(bin) + 1u);
        amplitudes[pixel] = (y1 - y0) * (bin - floor(bin)) + y0;
    }
}

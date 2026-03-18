// Waveform compute shader - calculates min/max bounds for each pixel column
struct Bounds {
    min_val: f32,
    max_val: f32,
}

@group(0) @binding(0) var<storage, read> signal: array<f32>;
@group(0) @binding(1) var<storage, read_write> bounds: array<Bounds>;

struct Uniforms {
    signal_length: u32,
    width: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(2) var<uniform> uniforms: Uniforms;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel = global_id.x;
    if (pixel >= uniforms.width) {
        return;
    }
    
    let step_size = f32(uniforms.signal_length) / f32(uniforms.width);
    let lower_index = u32(f32(pixel) * step_size);
    let upper_index = u32(f32(pixel + 1u) * step_size);
    
    // If the current pixel is between two points on the signal
    if (upper_index == lower_index) {
        let val = signal[pixel];
        bounds[pixel] = Bounds(val, val);
        return;
    }
    
    // Find min and max of the signal between the two points
    var min_value: f32 = 1.0;
    var max_value: f32 = -1.0;
    for (var i = lower_index; i < upper_index; i = i + 1u) {
        let val = signal[i];
        min_value = min(min_value, val);
        max_value = max(max_value, val);
    }
    bounds[pixel] = Bounds(min_value, max_value);
}

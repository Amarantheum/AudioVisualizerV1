// Spectrum render shader - draws the spectrum analyzer bars with anti-aliasing

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );
    
    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = positions[vertex_index];
    return out;
}

@group(0) @binding(0) var<storage, read> amplitudes: array<f32>;

struct RenderUniforms {
    width: u32,
    bar_width: f32,
    min_db: f32,
    max_db: f32,
    vertical_offset: f32,
    height: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(0) @binding(1) var<uniform> uniforms: RenderUniforms;

// Smooth interpolation for anti-aliasing
fn smoothstep_aa(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// Sample amplitude with linear interpolation between pixels
fn sample_amplitude(x: f32) -> f32 {
    let pixel_f = x * f32(uniforms.width);
    let pixel_i = u32(pixel_f);
    let frac = pixel_f - f32(pixel_i);
    
    let amp0 = amplitudes[min(pixel_i, uniforms.width - 1u)];
    let amp1 = amplitudes[min(pixel_i + 1u, uniforms.width - 1u)];
    
    return mix(amp0, amp1, frac);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x_norm = (in.uv.x + 1.0) / 2.0;
    
    // Sample amplitude with interpolation
    let amplitude = sample_amplitude(x_norm);
    
    // Normalize amplitude to 0-1 range based on dB range
    let normalized = (amplitude - uniforms.min_db) / (uniforms.max_db - uniforms.min_db);
    
    // Convert y position from -1..1 to 0..1 (bottom to top), then shift down by offset
    let y_normalized = (in.uv.y + 1.0) / 2.0 - uniforms.vertical_offset;
    
    // Screen Y position (0 at bottom, 1 at top) for color gradient
    let screen_y = (in.uv.y + 1.0) / 2.0;
    
    // Anti-aliasing: smooth transition at the edge (about 2 pixels worth)
    let aa_width = 2.0 / uniforms.height;
    let edge_factor = 1.0 - smoothstep_aa(normalized - aa_width, normalized + aa_width, y_normalized);
    
    // Color gradient based on screen Y position (not bar height)
    // Purple/magenta at bottom, cyan in middle, white/bright at top
    let r = 0.3 + screen_y * 0.7;
    let g = 0.1 + screen_y * 0.9;
    let b = 0.9 - screen_y * 0.1;
    
    // Apply edge factor for smooth anti-aliased edge
    let color = vec3<f32>(r, g, b) * edge_factor;
    
    // Add subtle glow above the bar
    let glow_dist = y_normalized - normalized;
    let glow_width = 0.05;
    if (glow_dist > 0.0 && glow_dist < glow_width) {
        let glow = (1.0 - glow_dist / glow_width) * 0.3;
        return vec4<f32>(color + vec3<f32>(glow * 0.5, glow * 0.8, glow), 1.0);
    }
    
    return vec4<f32>(color, 1.0);
}

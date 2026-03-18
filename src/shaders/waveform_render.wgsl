// Waveform render shader - draws the waveform with anti-aliasing and color gradient

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full-screen quad vertices
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

struct Bounds {
    min_val: f32,
    max_val: f32,
}

@group(0) @binding(0) var<storage, read> bounds: array<Bounds>;

struct RenderUniforms {
    width: u32,
    line_width: f32,
    height: f32,
    _padding: f32,
}

@group(0) @binding(1) var<uniform> uniforms: RenderUniforms;

// Smooth interpolation for anti-aliasing
fn smoothstep_aa(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// Sample bounds with linear interpolation between pixels
fn sample_bounds(x: f32) -> Bounds {
    let pixel_f = x * f32(uniforms.width);
    let pixel_i = u32(pixel_f);
    let frac = pixel_f - f32(pixel_i);
    
    let b0 = bounds[min(pixel_i, uniforms.width - 1u)];
    let b1 = bounds[min(pixel_i + 1u, uniforms.width - 1u)];
    
    var result: Bounds;
    result.min_val = mix(b0.min_val, b1.min_val, frac);
    result.max_val = mix(b0.max_val, b1.max_val, frac);
    return result;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x_norm = (in.uv.x + 1.0) / 2.0;
    
    // Sample bounds with interpolation for smoother appearance
    let bound = sample_bounds(x_norm);
    
    // Calculate distance from the waveform line
    let center = (bound.min_val + bound.max_val) / 2.0;
    let half_thickness = (bound.max_val - bound.min_val) / 2.0 + uniforms.line_width;
    
    let dist_from_center = abs(in.uv.y - center);
    
    // Anti-aliased edge (about 2 pixels worth of smoothing)
    let aa_width = 2.0 / uniforms.height;
    let edge_factor = 1.0 - smoothstep_aa(half_thickness - aa_width, half_thickness + aa_width, dist_from_center);
    
    // Color based on amplitude (distance from center line)
    // Teal/cyan core with purple/magenta edges
    let amplitude = abs(center);
    let intensity = clamp(half_thickness * 2.0, 0.0, 1.0);
    
    // Core color: bright cyan
    let core_r = 0.2;
    let core_g = 0.9;
    let core_b = 1.0;
    
    // Edge color: purple/magenta
    let edge_r = 0.8;
    let edge_g = 0.3;
    let edge_b = 0.9;
    
    // Blend based on distance from center within the line
    let edge_blend = smoothstep_aa(0.0, half_thickness, dist_from_center);
    let r = mix(core_r, edge_r, edge_blend);
    let g = mix(core_g, edge_g, edge_blend);
    let b = mix(core_b, edge_b, edge_blend);
    
    // Apply edge factor and add glow
    let glow_factor = edge_factor * edge_factor; // Squared for softer falloff
    let color = vec3<f32>(r, g, b) * edge_factor;
    
    // Add subtle outer glow
    let glow_dist = dist_from_center - half_thickness;
    let glow_width = uniforms.line_width * 3.0;
    if (glow_dist > 0.0 && glow_dist < glow_width) {
        let glow = (1.0 - glow_dist / glow_width) * 0.15;
        return vec4<f32>(color + vec3<f32>(glow * 0.3, glow * 0.6, glow * 0.8), 1.0);
    }
    
    // Dark background with subtle gradient
    if (edge_factor < 0.01) {
        let bg_gradient = 0.02 + abs(in.uv.y) * 0.02;
        return vec4<f32>(bg_gradient * 0.1, bg_gradient * 0.15, bg_gradient * 0.2, 1.0);
    }
    
    return vec4<f32>(color, 1.0);
}

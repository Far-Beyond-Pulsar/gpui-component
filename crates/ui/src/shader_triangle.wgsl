// Minimal WGSL shader for a colored triangle

struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>
}

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> VSOut {
    var out: VSOut;
    out.position = vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

// Basic 3D shader for WGPU renderer

struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    time: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position
    let world_pos = uniforms.model * vec4<f32>(vertex.position, 1.0);
    out.world_position = world_pos.xyz;
    
    // Simple perspective projection
    let aspect = 800.0 / 600.0;
    let fov = 1.0; // ~60 degrees
    let near = 0.1;
    let far = 100.0;
    
    // Camera position
    let eye = vec3<f32>(0.0, 2.0, 5.0);
    let look_at = vec3<f32>(0.0, 0.0, 0.0);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    
    // View space
    let view_pos = world_pos.xyz - eye;
    
    // Simple perspective
    let depth = -view_pos.z;
    let x = view_pos.x / (depth * fov * aspect);
    let y = view_pos.y / (depth * fov);
    let z = (depth - near) / (far - near);
    
    out.clip_position = vec4<f32>(x, y, z, depth);
    
    // Transform normal
    out.normal = (uniforms.model * vec4<f32>(vertex.normal, 0.0)).xyz;
    out.color = vertex.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let normal = normalize(in.normal);
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Ambient + diffuse
    let ambient = 0.3;
    let lighting = ambient + diffuse * 0.7;
    
    let final_color = in.color * lighting;
    
    return vec4<f32>(final_color, 1.0);
}

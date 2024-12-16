@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4f {
    var positions = array<vec2f, 3>(
        vec2f(-1.0, -1.0),
        vec2f(3.0, -1.0), 
        vec2f(-1.0, 3.0)  
    );
    let pos = positions[vertex_index];
    return vec4f(pos, 0.0, 1.0);
}


struct Uniform {
    resolution: vec2u,
    mouse: vec2f,
    time: f32,
    dt: f32,
}
@group(0) @binding(0) var<uniform> uni: Uniform;

@fragment
fn fs_main(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    return vec4f(fract(uni.time + coord.xy / 600.0), 0.0, 1.0);
}
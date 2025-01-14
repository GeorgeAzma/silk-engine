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
@group(0) @binding(0) var img: texture_2d<f32>;

fn luma(col: vec3f) -> f32 {
    return dot(col, vec3f(0.2126, 0.7152, 0.0722));
}

fn sample(coord: vec2i) -> vec3f {
    return textureLoad(img, coord, 0).rgb;
}

@fragment
fn fs_main(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let p = vec2i(coord.xy);
    let c = luma(sample(p));
    let n = luma(sample(p + vec2i(0, 1)));
    let s = luma(sample(p + vec2i(0, -1)));
    let e = luma(sample(p + vec2i(1, 0)));
    let w = luma(sample(p + vec2i(-1, 0)));
    let max = max(n, max(s, max(e, w)));
    let min = min(n, min(s, min(e, w)));
    let dif = max - min; 
    return vec4f(0);
}
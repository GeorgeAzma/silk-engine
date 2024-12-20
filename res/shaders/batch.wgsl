struct BatchVertex {
    @location(0) pos: vec2f,
    @location(1) uv: vec2f,
    @location(2) color: vec4f,
    @location(3) stroke_color: vec4f,
    @location(4) stroke_width: f32,
    @location(5) roundness: f32,
}

struct VSOut {
    @builtin(position) coord: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
    @location(2) stroke_color: vec4f,
    @location(3) stroke_width: f32,
    @location(4) roundness: f32,    
}

@vertex
fn vs_main(in: BatchVertex) -> VSOut {
    var out: VSOut;
    out.coord = vec4f(in.pos, 0, 1);
    out.uv = in.uv;
    out.color = in.color;
    out.stroke_color = in.stroke_color;
    out.stroke_width = in.stroke_width;
    out.roundness = in.roundness;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4f {
    return vec4f(1.0, 1.0, 0.0, 1.0);
}
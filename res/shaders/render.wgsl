struct Vertex {
    @location(0) pos: vec2f,
    @location(1) scale: vec2f,
    @location(2) color: u32,
    @location(3) roundness: f32,
    @location(4) rotation: f32,
    @location(5) stroke_width: f32,
    @location(6) stroke_color: u32,
}

struct VSOut {
    @builtin(position) coord: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
    @location(2) roundness: f32,    
    @location(3) stroke_color: vec4f,
    @location(4) stroke_width: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vert_idx: u32, in: Vertex) -> VSOut {
    var out: VSOut;
    out.uv = vec2f(vec2u(vert_idx % 2u, vert_idx / 2u)) * 2.0 - 1.0;
    out.coord = vec4f(in.pos + out.uv * in.scale, 0, 1);
    out.color = unpack4x8unorm(in.color);
    out.roundness = in.roundness;
    out.stroke_width = in.stroke_width;
    out.stroke_color = unpack4x8unorm(in.stroke_color);
    return out;
}

fn round_rect(p: vec2f, r: f32) -> f32 {
	let q = abs(p) - 1.0 + r;
	return length(max(q, vec2f(0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4f {
    let rr = round_rect(in.uv, 0.5);
    
    return vec4f(in.color.rgb, in.color.a * step(rr, 0.0));
}
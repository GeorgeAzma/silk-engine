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
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
    @location(2) roundness: f32,    
    @location(3) stroke_color: vec4f,
    @location(4) stroke_width: f32,
    @location(5) scale: vec2f,
}

@group(0) @binding(0) var<uniform> res: vec2f;

@vertex
fn vs_main(@builtin(vertex_index) vert_idx: u32, in: Vertex) -> VSOut {
    var out: VSOut;
    out.uv = vec2f(vec2u(vert_idx % 2u, vert_idx / 2u)) * 2.0 - 1.0;
    let suv = out.uv * in.scale;
    let rot_uv = suv * cos(in.rotation) + vec2f(-1, 1) * suv.yx * res.yx / res * sin(in.rotation);
    out.pos = vec4f(in.pos + rot_uv, 0, 1);
    out.color = unpack4x8unorm(in.color);
    out.roundness = in.roundness;
    out.stroke_width = in.stroke_width;
    out.stroke_color = unpack4x8unorm(in.stroke_color);
    out.scale = in.scale * res;
    return out;
}

fn elongated_rrect(p: vec2f, r: f32, h: vec2f) -> f32 { 
	let q = abs(p) - h; 
	let a = max(q, vec2f(0.0)) - 1.0 + r;
	return length(max(a, vec2f(0))) + min(max(a.x, a.y), 0.0) - r + min(max(q.x, q.y), 0.0); 
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4f {
    var rr = 0.0;
    if in.roundness < 1.0 {
        let scl = in.scale / min(in.scale.x, in.scale.y);
        rr = elongated_rrect(in.uv * scl, in.roundness, scl - 1);
    } else {
        rr = length(in.uv) - 1.0;
    }
    let edge = step(rr, 0.0);
    let strk = step(rr + in.stroke_width, 0.0);
    let col = mix(in.stroke_color, in.color, strk); 
    return col * vec4f(1, 1, 1, edge);
}
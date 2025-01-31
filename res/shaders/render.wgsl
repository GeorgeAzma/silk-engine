struct Vertex {
    @location(0) pos: vec2f,
    @location(1) scale: vec2f,
    @location(2) color: u32,
    @location(3) roundness: f32,
    @location(4) rotation: f32,
    @location(5) stroke_width: f32,
    @location(6) stroke_color: u32,
    @location(7) tex_coord: vec2u, // packed whxy
}

struct VSOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
    @location(2) roundness: f32,    
    @location(3) stroke_color: vec4f,
    @location(4) stroke_width: f32,
    @interpolate(flat) @location(5) scale: vec2f,
    @interpolate(flat) @location(6) tex_coord: vec4u,
}

@group(0) @binding(0) var<uniform> res: vec2f;
@group(0) @binding(1) var atlas: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vert_idx: u32, in: Vertex) -> VSOut {
    var out: VSOut;
    let uv = vec2f(vec2u(vert_idx % 2u, vert_idx / 2u));
    out.uv = uv * 2.0 - 1.0;
    let suv = out.uv * in.scale;
    let rot_uv = suv * cos(in.rotation) + vec2f(-1, 1) * suv.yx * res.yx / res * sin(in.rotation);
    out.pos = vec4f((in.pos * 2.0 - 1.0) + rot_uv * 2.0, 0, 1);
    out.color = unpack4x8unorm(in.color);
    out.roundness = in.roundness;
    out.stroke_width = in.stroke_width;
    out.stroke_color = unpack4x8unorm(in.stroke_color);
    out.scale = in.scale * res;
    out.scale /= min(out.scale.x, out.scale.y);
    if in.tex_coord.x > 0 {
        out.tex_coord = vec4u(in.tex_coord.y >> 16, in.tex_coord.y, in.tex_coord.x >> 48, in.tex_coord.x >> 32) & vec4u(0xFFFF);
    } else {
        out.tex_coord = vec4u(~0u);
    }
    return out;
}

fn elongated_rrect(p: vec2f, r: f32, h: vec2f) -> f32 { 
	let q = abs(p) - h; 
	let a = max(q, vec2f(0)) - 1.0 + r;
	return length(max(a, vec2f(0))) + min(max(a.x, a.y), 0.0) - r + min(max(q.x, q.y), 0.0); 
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4f {
    // problems (hard):
    // - rounded rects have slight transparent edge
    // - edge flickering when smaller than couple pixels
    var r = 0.0;
    if in.roundness < 1.0 {
        r = elongated_rrect(in.uv * in.scale, in.roundness, in.scale - 1);
    } else {
        r = length(in.uv) - 1.0;
    }

    var d = max(abs(dpdx(r)), abs(dpdy(r)));
    r -= d * 0.5;
    d *= 1.5;
    let edge = saturate(1.0 - in.roundness * 0.75 - r / d);
    let strk = saturate((r + in.stroke_width) / d);
    var col = mix(in.color, in.stroke_color, strk);
    col.a *= edge;
    if in.tex_coord.x != ~0u {
        let p = vec2u((in.uv * 0.5 + 0.5) * vec2f(in.tex_coord.zw)) + in.tex_coord.xy;
        col *= textureLoad(atlas, p, 0);
    }
    if col.a < 0.001 {
        discard;
    }
    return col;
}
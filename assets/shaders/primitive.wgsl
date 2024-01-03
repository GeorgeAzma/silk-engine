struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) @interpolate(flat) color: vec4f,
    @location(2) @interpolate(flat) stroke_color: vec4f,
    @location(3) @interpolate(flat) stroke_width: f32,
    @location(4) @interpolate(flat) roundness: f32,
    @location(5) @interpolate(flat) side_ang: f32,
    @location(6) @interpolate(flat) scale: vec2f,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vert_id: u32,
    @location(0) position: vec3f,
    @location(1) scale: vec2f,
    @location(2) color: u32,
    @location(3) stroke_color: u32,
    @location(4) stroke_width: f32,
    @location(5) roundness: f32,
    @location(6) rotation: f32,
    @location(7) sides: i32,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2f(f32(vert_id % 2u), f32(vert_id / 2u)) * 2.0 - 1.0;
    let pos = (cos(rotation) * out.uv * scale + sin(rotation) * vec2f(out.uv.y * scale.y, -out.uv.x * scale.x)) + position.xy;
    out.clip_position = vec4f(pos, position.z, 1.0);
    out.color = unpack4x8unorm(color);
    out.stroke_color = unpack4x8unorm(stroke_color);
    out.stroke_width = stroke_width;
    out.roundness = roundness;
    out.side_ang = 3.141592653589793 / f32(sides);
    out.scale = scale;
    return out;
}

fn sdf_ngon(uv: vec2f, side_ang: f32, roundness: f32) -> f32 {
    let rnd = 2.0 / clamp(1.0 - roundness, 1e-5, 1.0) - 2.0;
    let r = 1.0;
    var p = uv * (1.0 + rnd / r);
    let he = r * tan(side_ang);
    p = -p.yx;
    let bn = 2. * side_ang * floor((atan2(p.y, p.x) + side_ang) / side_ang * .5);
    let cs = vec2f(cos(bn), sin(bn));
    p = mat2x2(cs.x, -cs.y, cs.y, cs.x) * p;
    return (length(p - vec2(r, clamp(p.y, -he, he))) * sign(p.x - r) - rnd) / (1.0 + rnd);
}

fn elongate(p: vec2f, h: vec2f) -> vec3f
{
    let q = abs(p) - h;
    return vec3f(max(q, vec2f(0.0)), min(max(q.x, q.y), 0.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var d = 0.0;
    let scl = in.scale / in.scale.yx;
    if scl.x > 1.0 {
        let w = elongate(in.uv, vec2f(1.0 - scl.y, 0.0));
        d = w.z + sdf_ngon(w.xy * vec2f(scl.x, 1.0), in.side_ang, in.roundness);
    } else {
        let w = elongate(in.uv, vec2f(0.0, 1.0 - scl.x));
        d = w.z + sdf_ngon(w.xy * vec2f(1.0, scl.y), in.side_ang, in.roundness);
    }
    let dd = length(vec2f(dpdx(d), dpdy(d))) * 1.5;
    var color = mix(in.color, in.stroke_color, smoothstep(-in.stroke_width, -in.stroke_width + dd, d));
    color.a *= smoothstep(0.0, -dd, d); 
    return color;
}
 
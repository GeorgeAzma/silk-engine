struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2f,
    @location(1) @interpolate(flat) color: vec4f,
    @location(2) @interpolate(flat) stroke_color: vec4f,
    @location(3) @interpolate(flat) stroke_width: f32,
    @location(4) @interpolate(flat) roundness: f32,
    @location(5) @interpolate(flat) sides: i32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vert_id: u32,
    @location(0) position: vec2f,
    @location(1) scale: vec2f,
    @location(2) color: vec4f,
    @location(3) stroke_color: vec4f,
    @location(4) stroke_width: f32,
    @location(5) roundness: f32,
    @location(6) rotation: f32,
    @location(7) sides: i32,
) -> VertexOutput {
    var out: VertexOutput;
    let rec = vec2f(f32(vert_id % 2u), f32(vert_id / 2u)) * 2.0 - 1.0;
    let pos = (cos(rotation) * rec + sin(rotation) * vec2f(rec.y, -rec.x)) * scale + position;
    out.clip_position = vec4f(pos, 0.0, 1.0);
    out.uv = rec;
    out.color = color;
    out.stroke_color = stroke_color;
    out.stroke_width = stroke_width;
    out.roundness = roundness;
    out.sides = sides;
    return out;
}

const PI: f32 = 3.141592653589793; 

fn sdf_ngon(uv: vec2f, sides: i32, roundness: f32) -> f32 {
    let an = PI / f32(sides);
    let r = cos(an);
    var p = uv * (1.0 + roundness / r);
    let he = r * tan(an);
    p = -p.yx;
    let bn = 2. * an * floor((atan2(p.y, p.x) + an) / an * .5);
    let cs = vec2f(cos(bn), sin(bn));
    p = mat2x2(cs.x, -cs.y, cs.y, cs.x) * p;
    return (length(p - vec2(r, clamp(p.y, -he, he))) * sign(p.x - r) - roundness) / (1.0 + roundness);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var color = in.color;
    let f = length(vec2f(dpdx(in.uv.x), dpdy(in.uv.y))) * 2.;
    let d = sdf_ngon(in.uv, in.sides, in.roundness);
    color = mix(color, in.stroke_color, smoothstep(-in.stroke_width, -in.stroke_width + f, d));
    color.a *= smoothstep(0.0, -f, d);
    return color;
}
 
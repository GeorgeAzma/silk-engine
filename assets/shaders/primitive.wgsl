struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) @interpolate(flat) color: vec4f,
    @location(2) @interpolate(flat) stroke_color: vec4f,
    @location(3) @interpolate(flat) stroke_width: f32,
    @location(4) @interpolate(flat) roundness: f32,
    @location(5) @interpolate(flat) side_ang: f32,
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
    let pos = (cos(rotation) * out.uv + sin(rotation) * vec2f(out.uv.y, -out.uv.x)) * scale + position.xy;
    out.clip_position = vec4f(pos, position.z, 1.0);
    out.color = unpack4x8unorm(color);
    out.stroke_color = unpack4x8unorm(stroke_color);
    out.stroke_width = stroke_width;
    out.roundness = roundness;
    out.side_ang = 3.141592653589793 / f32(sides);
    return out;
}

fn sdf_ngon(uv: vec2f, side_ang: f32, roundness: f32) -> f32 {
    let r = cos(side_ang);
    var p = uv * (1.0 + roundness / r);
    let he = r * tan(side_ang);
    p = -p.yx;
    let bn = 2. * side_ang * floor((atan2(p.y, p.x) + side_ang) / side_ang * .5);
    let cs = vec2f(cos(bn), sin(bn));
    p = mat2x2(cs.x, -cs.y, cs.y, cs.x) * p;
    return (length(p - vec2(r, clamp(p.y, -he, he))) * sign(p.x - r) - roundness) / (1.0 + roundness);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var color = in.color;
    let d = sdf_ngon(in.uv, in.side_ang, in.roundness);
    let f = length(fwidth(in.uv));
    color = mix(color, in.stroke_color, smoothstep(-in.stroke_width, -in.stroke_width + f, d));
    color.a *= smoothstep(0.0, -f, d);
    return color;
}
 
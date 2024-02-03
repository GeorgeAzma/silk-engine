struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) @interpolate(flat) color: vec4f,
    @location(2) @interpolate(flat) stroke_color: vec4f,
    @location(3) @interpolate(flat) stroke_width: f32,
    @location(4) @interpolate(flat) texcoord: vec4f,
    @location(5) @interpolate(flat) bold: f32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vert_id: u32,
    @location(0) position: vec2f,
    @location(1) scale: vec2f,
    @location(2) color: u32,
    @location(3) stroke_color: u32,
    @location(4) stroke_width: f32,
    @location(5) rotation: f32,
    @location(6) texcoord: vec4f,
    @location(7) bold: f32,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = (vec2f(f32(vert_id % 2u), f32(vert_id / 2u)) * 2.0 - 1.0) * 1.75;
    let pos = (cos(rotation) * out.uv + sin(rotation) * vec2f(out.uv.y, -out.uv.x)) * scale + position.xy;
    out.clip_position = vec4f(pos, 0.0, 1.0);
    out.color = unpack4x8unorm(color);
    out.stroke_color = unpack4x8unorm(stroke_color);
    out.stroke_width = stroke_width;
    out.texcoord = texcoord;
    out.bold = bold;
    return out;
}

@group(0) @binding(0) var t_atlas: texture_2d<f32>;
@group(0) @binding(1) var s_atlas: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var color = in.color;
    let d = (textureSample(t_atlas, s_atlas, (in.uv * vec2f(0.5, -0.5) + vec2f(0.5, -0.5)) * in.texcoord.zw + in.texcoord.xy).r + in.bold * 0.25) / 1.41421356;
    let dd = length(vec2f(dpdx(d), dpdy(d))) * 1.5;
    color = mix(color, in.stroke_color, clamp((0.5 - d + in.stroke_width * 0.5) / dd, 0.0, 1.0));
    color.a = clamp((d - 0.5) / dd + 0.5, 0.0, 1.0);
    return color;
}
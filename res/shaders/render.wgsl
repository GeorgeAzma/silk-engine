struct Vertex {
    @location(0) pos: vec2f,
    @location(1) scale: vec2f,
    @location(2) color: u32,
    @location(3) roundness: f32,
    @location(4) rotation: f32, // negative is bold
    @location(5) stroke_width: f32,
    @location(6) stroke_color: u32,
    @location(7) tex_coord: vec2u, // packed whxy
    @location(8) blur: f32,
    @location(9) stroke_blur: f32,
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
    @interpolate(flat) @location(7) blur: f32,
    @interpolate(flat) @location(8) stroke_blur: f32,
}

@group(0) @binding(0) var<uniform> res: vec2f;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vert_idx: u32, in: Vertex) -> VSOut {
    var out: VSOut;
    var uv = vec2f(vec2u(vert_idx % 2u, vert_idx / 2u));
    out.uv = (uv * 2.0 - 1.0) * (1.05 + max(0.17 * abs(in.blur), -(in.roundness + 1.0) * 0.1));
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
    out.blur = in.blur;
    out.stroke_blur = in.stroke_blur;
    return out;
}

fn elongated_rrect(p: vec2f, r: f32, h: vec2f) -> f32 { 
	let q = abs(p) - h; 
	let a = max(q, vec2f(0)) - 1.0 + r;
	return length(max(a, vec2f(0))) + min(max(a.x, a.y), 0.0) - r + min(max(q.x, q.y), 0.0); 
}

fn render(in: VSOut, off: vec2f, blur: f32) -> vec2f {
    let uv = in.uv + off;
    if in.roundness < 0.0 {
        let bold = -(1.0 + in.roundness);
        let p = ((uv / 1.125 * 0.5 + 0.5) * vec2f(in.tex_coord.zw) + vec2f(in.tex_coord.xy)) / vec2f(textureDimensions(atlas).xy);
        var r = 2.0 * (0.5 - textureSample(atlas, atlas_sampler, p).r * (bold * 0.5 + 1.0));
        let pd = vec3f(dpdx(p.x), dpdy(p.y), 0.0);
        var px = textureSample(atlas, atlas_sampler, p + pd.xz).r;
        var py = textureSample(atlas, atlas_sampler, p + pd.zy).r;
        var nx = textureSample(atlas, atlas_sampler, p - pd.xz).r;
        var ny = textureSample(atlas, atlas_sampler, p - pd.zy).r;
        let d = max(abs(px - nx), abs(py - ny)) * (bold * 0.5 + 1.0);
        // let d = max(abs(dpdx(r)), abs(dpdy(r)));
        let edge = saturate(1.0 - r / mix(d, 1.0, blur));
        let strk = select(saturate((r + in.stroke_width * 0.5 * (1.0 + bold)) / mix(d, 1.0, in.stroke_blur + blur)), 0.0, in.stroke_width < 0.001);
        return vec2f(edge, strk);
    } else {
        var r = 0.0;
        if in.roundness < 1.0 {
            r = elongated_rrect(uv * in.scale, in.roundness, in.scale - 1);
        } else {
            r = length(uv) - 1.0;
        }
        var d = max(abs(dpdx(r)), abs(dpdy(r)));
        r -= d * 0.5;
        d *= 1.5;
        let edge = saturate(1.0 - in.roundness * 0.75 - r / mix(d, 0.5, blur));
        let strk = select(saturate((r + in.stroke_width * 1.05) / mix(d, 0.5, in.stroke_blur + blur)), 0.0, in.stroke_width < 0.001);
        return vec2f(edge, strk);
    }
    return vec2f(0);
}

// problems (hard):
// - rounded rects have slight transparent edge
// - edge flickering when smaller than couple pixels
@fragment
fn fs_main(in: VSOut) -> @location(0) vec4f {
    // hacky way to render text
        // [-1, 0, 1] = [thin, normal, bold]
        let blur = max(0.0, in.blur);
        var esg = render(in, vec2f(0), blur);
        var col = vec4f(0);
        // supersampling for non-blurred render
        if in.blur <= 0.0 {
            let dx = length(dpdx(in.uv)) / 3.0;
            let esxr = render(in, vec2f(-dx, 0), blur);
            let esxb = render(in, vec2f(dx, 0), blur);
            let dy = length(dpdy(in.uv)) / 3.0;
            let esyr = render(in, vec2f(0, -dy), blur);
            let esyb = render(in, vec2f(0, dy), blur);
            esg = (esg + esxr + esxb + esyr + esyb) / 5.0;
            if in.blur < 0.0 {
                let glow = render(in, vec2f(0), -in.blur);
                esg.y = mix(glow.y, 1.0, esg.y);
                esg.x = mix(glow.x, 1.0, 1.0-esg.y);
            }
            col = mix(in.color, in.stroke_color, esg.y);
            col.a *= esg.x;
        } else {
            col = mix(in.color, in.stroke_color, esg.y);
            col.a *= esg.x;
        }
        if in.roundness >= 0.0 && in.tex_coord.x != ~0u {
            let p = vec2u((in.uv * 0.5 + 0.5) * vec2f(in.tex_coord.zw)) + in.tex_coord.xy;
            col *= textureLoad(atlas, p, 0);
        }
        if col.a < 0.001 {
            discard;
        }
        return col;
}
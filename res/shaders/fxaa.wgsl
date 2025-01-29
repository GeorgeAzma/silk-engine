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

@group(0) @binding(0) var img: texture_2d<f32>;
@group(0) @binding(1) var img_sampler: sampler;

fn luma(col: vec3f) -> f32 {
    return dot(col, vec3f(0.299, 0.587, 0.114));
}

fn sample(coord: vec2i) -> vec4f {
    return textureLoad(img, coord, 0);
}

fn sample_luma(coord: vec2i) -> f32 {
    return luma(sample(coord).rgb);
}

const MIN_THRESHOLD: f32 = 1.0 / 24.0;
const MAX_THRESHOLD: f32 = 1.0 / 12.0;
const ITERS: u32 = 8;

@fragment
fn fs_main(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let p = vec2i(coord.xy);
    
    let col = sample(p);
    if true {
        return col;
    }
    let c = luma(col.rgb);

    let n = sample_luma(p + vec2i( 0,  1));
    let s = sample_luma(p + vec2i( 0, -1));
    let e = sample_luma(p + vec2i( 1,  0));
    let w = sample_luma(p + vec2i(-1,  0));
    
    let min = min(c, min(min(n, s), min(e, w)));
    let max = max(c, max(max(n, s), max(e, w)));
    let rng = max - min;
    if rng < max(MIN_THRESHOLD, max * MAX_THRESHOLD) {
        return col;
    }

    let ne = sample_luma(p + vec2i( 1,  1));
    let nw = sample_luma(p + vec2i(-1,  1));
    let se = sample_luma(p + vec2i( 1, -1));
    let sw = sample_luma(p + vec2i(-1, -1));
    
    let horz = abs(nw + sw - 2 * w) + abs(s + n - 2 * c) * 2 + abs(ne + se - 2 * e);
    let vert = abs(nw + ne - 2 * n) + abs(e + w - 2 * c) * 2 + abs(sw + se - 2 * s);
    let is_horz = horz >= vert;
    let luma = vec2f(select(vec2f(w, e), vec2f(s, n), is_horz)); 
    let grad = vec2f(luma - c);
    let grad_scl = 0.25 * max(abs(grad.x), abs(grad.y));
    let ires = 1.0 / vec2f(textureDimensions(img));
    var step = select(ires.x, ires.y, is_horz);
    var avg = 0.0;
    if abs(grad.x) >= abs(grad.y) {
        step *= -1.0;
        avg = mix(luma.x, c, 0.5);
    } else {
        avg = mix(luma.y, c, 0.5);
    }
    let uv = coord.xy * ires;
    var cur_uv = uv;
    if is_horz {
        cur_uv.y += step * 0.5;
    } else {
        cur_uv.x += step * 0.5;
    }
    let off = select(vec2f(0, ires.y), vec2f(ires.x, 0), is_horz);
    var uv1 = cur_uv - off;
    var uv2 = cur_uv + off;
    var end1 = luma(textureSample(img, img_sampler, uv1).rgb) - avg;
    var end2 = luma(textureSample(img, img_sampler, uv2).rgb) - avg;
    var reached1 = abs(end1) >= grad_scl;
    var reached2 = abs(end2) >= grad_scl;
    for (var i = 0u; i < ITERS && !(reached1 && reached2); i += 1u) {
        if !reached1 {
            uv1 -= off;
            end1 = luma(textureSample(img, img_sampler, uv1).rgb) - avg;
            reached1 = abs(end1) >= grad_scl;
        }
        if !reached2 {
            uv2 += off;
            end2 = luma(textureSample(img, img_sampler, uv2).rgb) - avg;
            reached2 = abs(end2) >= grad_scl;
        }
    }
    let d = select(vec2f(uv.y - uv1.y, uv2.y - uv.y), 
                   vec2f(uv.x - uv1.x, uv2.x - uv.x), is_horz);
    let px_off = 0.5 - min(d.x, d.y) / (d.x + d.y);
    let corr_var = (select(end2, end1, d.x < d.y) < 0.0) != (c < avg);
    var final_off = px_off * f32(corr_var);
    
    // let luma_avg = (2 * (n + s + w + e) + nw + ne + sw + se) / 12;
    // let subpx_off1 = smoothstep(0.0, 1.0, abs(luma_avg - c) / rng);
    // let subpx_off = subpx_off1 * subpx_off1 * 0.75;
    final_off = max(final_off, /*subpx_off*/ 0.0);
    var final_uv = uv;
    if is_horz  {
        final_uv.y += final_off * step;
    } else {
        final_uv.x += final_off * step;
    }
    return textureSample(img, img_sampler, final_uv);
}
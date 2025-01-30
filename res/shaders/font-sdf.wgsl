// # How it works
// for each glyph:
//   for each pixel / 4:
//     run invocation:
//       for 1..4:
//         for each bezier:
//           calc sdf (0-255)
//           and pack result in u32
// note: efficient if font has similar bezier count per glyph

struct Uniform {
    sdf_width: u32,
    char_size_px: u32,
}

@group(0) @binding(0) var<storage, read_write> sdf: array<u32>;
@group(0) @binding(1) var<uniform> uni: Uniform;
@group(0) @binding(2) var<storage, read> points: array<vec2f>; // 0-1 range
// packed (off, size, wh, xy) per glyph
@group(0) @binding(3) var<storage, read> glyphs: array<vec4u>;

fn cross2(a: vec2f, b: vec2f) -> f32 {
    return a.x * b.y - a.y * b.x;
}

// https://www.shadertoy.com/view/ftdGDB
fn bezier_sdf(p: vec2f, A: vec2f, B: vec2f, C: vec2f) -> f32 {
    let EPS = 1e-6;
    let a = B - A;
    let b = A - 2.0 * B + C;
    let c = a * 2.0;
    let d = A - p;

    let kk = 1.0 / dot(b, b);
    let kx = kk * dot(a, b);
    let ky = kk * (2.0 * dot(a, a) + dot(d, b)) / 3.0;
    let kz = kk * dot(d, a);

    var res = 0.0;
    var sgn = 0.0;

    let p1 = ky - kx * kx;
    let p3 = p1 * p1 * p1;
    let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
    var h = q * q + 4.0 * p3;
    if h >= 0.0 {
        h = sqrt(h);
        let x = 0.5 * (vec2f(h, -h) - q);
        let uv = sign(x) * pow(abs(x), vec2f(1.0 / 3.0));
        let t = saturate(uv.x + uv.y - kx) + EPS;
        let q = d + (c + b * t) * t;
        res = dot(q, q);
        sgn = cross2(c + 2.0 * b *  t, q);
    } else {
        let z = sqrt(-p1);
        let v = acos(q / (p1 * z * 2.0)) / 3.0;
        let m = cos(v);
        let n = sin(v) * sqrt(3.0);
        let t = saturate(vec3f(m + m, -n - m, n - m) * z-kx) + EPS;
        let qx = d + (c + b * t.x) * t.x;
        let dx = dot(qx, qx);
        let sx = cross2(c + 2.0 * b * t.x, qx);
        let qy = d + (c + b * t.y) * t.y;
        let dy = dot(qy, qy);
        let sy = cross2(c + 2.0 * b * t.y, qy);
        res = select(dy, dx, dx < dy);
        sgn = select(sy, sx, dx < dy);
    }
    return sign(sgn) * sqrt(res);
}

@compute
@workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    // global_invocation_id = (px_id / 4, glyph_id)
    if gid.y >= arrayLength(&glyphs) || gid.x >= arrayLength(&sdf) {
        return;
    }
    let glyph = glyphs[gid.y];
    let gs = vec2u(glyph.z >> 16, glyph.z & 0xFFFF);
    let gp = vec2u(glyph.w >> 16, glyph.w & 0xFFFF);
    var id = gid.x * 4;
    let pu = vec2u(id % uni.sdf_width, id / uni.sdf_width);
    if any(vec4(pu < gp, pu >= gp + gs)) {
        return;
    }
    for (var j = 0u; j < 4; j += 1u) {
        let idx = id + j;
        let puj = vec2u(idx % uni.sdf_width, idx / uni.sdf_width);
        let p = vec2f(puj - gp) / vec2f(uni.char_size_px);
        var d = 9999.0;
        for (var i = 0u; i < glyph.y; i += 1u) {
            let off = glyph.x + i * 3;
            let a = points[off + 0];
            let b = points[off + 1];
            let c = points[off + 2];
            let dir = normalize(c - a) * 5e-5;
            let bd = bezier_sdf(p, a + dir, b + dir, c - dir);
            if abs(bd) < abs(d) {
                d = bd;
            }
        }
        sdf[gid.x] |= u32(saturate(d / 0.5 + 0.5) * 255.0) << (j * 8u);
    }

}
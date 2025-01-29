// # How it works
// for each glyph:
//   for each pixel / 4:
//     run invocation:
//       for 1..4:
//         for each bezier:
//           calc sdf (0-255) for 4 pixels at a time
//           and pack result in u32
// notes: 
// - efficient if font has similar bezier count per glyph
// - result has to be unpacked into an R8 image for storage

struct Uniform {
    sdf_width: u32,
    max_glyph_width: u32,
    max_glyph_height: u32,
}

@group(0) @binding(0) var<storage, read_write> sdf: array<u32>;
@group(0) @binding(1) var<uniform> sdf_width: u32;
@group(0) @binding(2) var<storage, read> points: array<vec2f>; // 0-1 range
// packed (off, size, wh, xy) per glyph
@group(0) @binding(3) var<storage, read> glyphs: array<vec4u>;

fn cross(a: vec2f, b: vec2f, p: vec2f) -> f32 {
    return dot(b - a, vec2f(a.y - p.y, p.x - a.x));
}

fn sign_bezier(A: vec2f, B: vec2f, C: vec2f, p: vec2f) -> f32 { 
    let a = C - A;
    let b = vec2f(B.y - A.y, A.x - B.x);
    let c = p - A;
    let bary = vec2f(dot(c, b), dot(a, vec2f(c.y, -c.x))) / dot(a, b);
    let d = vec2f(bary.y * 0.5, 0.0) + 1.0 - bary.x - bary.y;
    return sign(((d.x * d.x - d.y) * step(d.y, d.x) + cross(B, A, p) * cross(B, C, p) * step(d.x, d.y)) * dot(a, -b));
}

fn solve_cubic(a: f32, b: f32, c: f32) -> vec3f {
    let p = b - a * a / 3.0;
    let p3 = p * p * p;
    let q = a * (2.0 * a * a - 9.0 * b) / 27.0 + c;
    let d = q * q + 4.0 * p3 / 27.0;
    let offset = -a / 3.0;
    if(d >= 0.0) { 
        let z = sqrt(d);
        let x = (vec2f(z, -z) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2f(1.0 / 3.0));
        return vec3f(offset + uv.x + uv.y);
    }
    let v = acos(-sqrt(-27.0 / p3) * q / 2.0) / 3.0;
    let m = cos(v);
    let n = sin(v) * sqrt(3.0);
    return vec3f(m + m, -n - m, n - m) * sqrt(-p / 3.0) + offset;
}

fn bezier_sdf(A: vec2f, B: vec2f, C: vec2f, p: vec2f) -> f32 {    
    let b1 = B + 0.00001;
    let a2 = b1 - A;
    let b2 = A - b1 * 2.0 + C;
    let c2 = a2 * 2.0;
    let d = A - p;
    let k = vec3f(3.0 * dot(a2, b2), 2.0 * dot(a2, a2) + dot(d, b2), dot(d, a2)) / dot(b2, b2);      
    let t = clamp(solve_cubic(k.x, k.y, k.z), vec3f(0.0), vec3f(1.0));
    var pos = A + (c2 + b2 * t.x) * t.x;
    var dis = length(pos - p);
    pos = A + (c2 + b2 * t.y) * t.y;
    dis = min(dis, length(pos - p));
    pos = A + (c2 + b2 * t.z) * t.z;
    dis = min(dis, length(pos - p));
    return dis * sign_bezier(A, b1, C, p);
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
    let id = gid.x * 4;
    let p = vec2u(id % sdf_width, id / sdf_width);
    if any(vec4(p < gp, p >= gp + gs)) {
        return;
    }
    let off = glyph.x;
    let bezier_count = glyph.y;
    for (var i = 0u; i < 4; i += 1u) {
        let idx = id + i;
        let pu = vec2u(idx % sdf_width, idx / sdf_width);
        let p = vec2f(pu - gp) / vec2f(256, 256);
        var d = 9999.0;
        var ds = 9999.0;
        for (var j = 0u; j < bezier_count; j += 1u) {
            let a = points[j * 3 + off + 0];
            let b = points[j * 3 + off + 1];
            let c = points[j * 3 + off + 2];
            let dir = normalize(c - a) * 0.00001;
            let bd = bezier_sdf(a + dir, b + dir * vec2f(10), c - dir, p);
            if abs(bd) < abs(d) {
                d = bd;
            }
        }
        let sdfu = u32(round(smoothstep(-0.3, 0.3, d) * 255.0));
        sdf[gid.x] |= sdfu << (i * 8u);
    }
}
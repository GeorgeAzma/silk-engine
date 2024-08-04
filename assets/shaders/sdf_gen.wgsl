@group(0) @binding(0) var<storage, read_write> sdf: array<u32>;

struct Curve {
    a: vec2f,
    b: vec2f,
    c: vec2f,
}

@group(0) @binding(1) var<storage, read> curves: array<Curve>;

struct Glyph {
    offset: u32,
    size: u32,
    res: vec2u,
    uv: vec4f,
} 

@group(0) @binding(2) var<storage, read> glyphs: array<Glyph>;

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
    let b1 = B + vec2f(0.0001);
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
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let id = global_id.x / 4u;
    let glyph = glyphs[global_id.y];
    let gp = vec2f(f32((id * 4u) % glyph.res.x), f32(glyph.res.y - (id * 4u) / glyph.res.x)) / vec2f(glyph.res);
    if any(gp < glyph.uv.xy - glyph.uv.zw * vec2f(0.2, 0.3)) || any(gp > glyph.uv.xy + glyph.uv.zw) {
        return;
    }
    for (var i = 0u; i < 4u; i += 1u) {
        let cd = id * 4u + i;
        let p = (vec2f(f32(cd % glyph.res.x), f32(glyph.res.y - cd / glyph.res.x)) / vec2f(glyph.res) - glyph.uv.xy) / glyph.uv.zw;
        var d = 10000.0;
        for (var j = 0u; j < glyph.size; j += 1u) {
            let curve = curves[j + glyph.offset];
            let dir = normalize(curve.c - curve.a) * 0.002;
            let d1 = bezier_sdf(curve.a + dir, curve.b, curve.c - dir, p);
            if abs(d1) < abs(d) {
                d = d1;
            }
        }
        sdf[id] |= min(u32(round(smoothstep(-0.3, 0.3, d) * 255.0 * 1.41421356)), 255u) << (i * 8u);
    }
}
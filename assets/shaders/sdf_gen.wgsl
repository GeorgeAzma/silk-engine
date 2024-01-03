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

fn test_cross(a: vec2f, b: vec2f, p: vec2f) -> f32 {
    return sign((b.y - a.y) * (p.x - a.x) - (b.x - a.x) * (p.y - a.y));
}

fn sign_bezier(A: vec2f, B: vec2f, C: vec2f, p: vec2f) -> f32 { 
    let a = C - A;
    let b = B - A;
    let c = p - A;
    let bary = vec2(c.x * b.y - b.x * c.y, a.x * c.y - c.x * a.y) / (a.x * b.y - b.x * a.y);
    let d = vec2(bary.y * 0.5, 0.0) + 1.0 - bary.x - bary.y;
    return mix(sign(d.x * d.x - d.y), mix(-1.0, 1.0, 
        step(test_cross(A, B, p) * test_cross(B, C, p), 0.0)),
        step((d.x - d.y), 0.0)) * test_cross(A, C, B);
}

fn solve_cubic(a: f32, b: f32, c: f32) -> vec3f
{
    let p = b - a * a / 3.0;
    let p3 = p * p * p;
    let q = a * (2.0 * a * a - 9.0 * b) / 27.0 + c;
    let d = q*q + 4.0 * p3 / 27.0;
    let offset = -a / 3.0;
    if(d >= 0.0) { 
        let z = sqrt(d);
        let x = (vec2(z, -z) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2(1.0/3.0));
        return vec3(offset + uv.x + uv.y);
    }
    let v = acos(-sqrt(-27.0 / p3) * q / 2.0) / 3.0;
    let m = cos(v);
    let n = sin(v) * 1.73205080757;
    return vec3(m + m, -n - m, n - m) * sqrt(-p / 3.0) + offset;
}

fn bezier_sdf(A: vec2f, B: vec2f, C: vec2f, p: vec2f) -> f32 {    
    let b1 = mix(B + vec2(1e-4), B - vec2(1e-4), abs(sign(B * 2.0 - A - C)));
    let a2 = b1 - A;
    let b2 = A - b1 * 2.0 + C;
    let c2 = a2 * 2.0;
    let d = A - p;
    let k = vec3(3. * dot(a2, b2), 2. * dot(a2, a2) + dot(d, b2), dot(d, a2)) / dot(b2, b2);      
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
    let gp = glyph.uv.xy + glyph.uv.zw * 0.5;
    let p = vec2f(f32((id * 4u) % glyph.res.x), f32(glyph.res.y - (id * 4u) / glyph.res.x)) / vec2f(glyph.res);
    if abs(gp.y - p.y) > glyph.uv.w || abs(gp.x - p.x) > glyph.uv.z {
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
        sdf[id] |= clamp(u32(round(smoothstep(-0.3, 0.3, d) * 255.0 * 1.41421356)), 0u, 255u) << (i * 8u);
    }
}
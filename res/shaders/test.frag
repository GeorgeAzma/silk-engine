#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) flat in vec4 color;
layout(location = 2) flat in float roundness;
layout(location = 3) flat in vec4 stroke_color;
layout(location = 4) flat in float stroke_width;
layout(location = 5) flat in vec2 scale;
layout(location = 6) flat in uvec4 tex_coord;
layout(location = 7) flat in float blur;
layout(location = 8) flat in float stroke_blur;
layout(location = 9) flat in vec4 gradient_color;
layout(location = 10) flat in vec2 gradient_dir;
layout(location = 11) flat in float superellipse;

layout(location = 0) out vec4 out_color;

float superellipse_corner(vec2 p, float r, float n) {
    p = abs(p);
    float v = pow(pow(p.x, n) + pow(p.y, n), 1.0 / n);
    return v - r;
}

float superellipse_sdf(vec2 p, vec2 size, float corner_radius, float n) {
    vec2 d = abs(p) - size;
    if(d.x > -corner_radius && d.y > -corner_radius) {
        vec2 corner_center = sign(p) * (size - vec2(corner_radius));
        return superellipse_corner(p - corner_center, corner_radius, n);
    } else {
        return min(max(d.x, d.y), 0.0) + length(max(d, 0.0));
    }
}

float elongate(vec2 p, float r, vec2 h) {
    vec2 q = abs(p) - h;
    return superellipse_sdf(sign(p) * max(q, vec2(0)), vec2(1), r, exp2(superellipse)) + min(max(q.x, q.y), 0.0);
}

float elongated_rrect(vec2 p, float r, vec2 h) {
    return elongate(p, r, h);
    vec2 q = abs(p) - h;
    vec2 a = max(q, vec2(0)) - 1.0 + r;
    return length(max(a, vec2(0))) + min(max(a.x, a.y), 0.0) - r + min(max(q.x, q.y), 0.0);
}

vec3 lrgb2srgb(vec3 lrgb) {
    return mix(12.92 * lrgb, 1.055 * pow(lrgb, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), lrgb));
}

vec4 oklab_mix(vec4 lin1, vec4 lin2, float a) {
    if(a <= 0.0) {
        return lin1;
    } else if(a >= 1.0) {
        return lin2;
    }
    mat3 cone2lms = mat3(0.4121656120, 0.2118591070, 0.0883097947, 0.5362752080, 0.6807189584, 0.2818474174, 0.0514575653, 0.1074065790, 0.6302613616);
    mat3 lms2cone = mat3(4.0767245293, -1.2681437731, -0.0041119885, -3.3072168827, 2.6093323231, -0.7034763098, 0.2307590544, -0.3411344290, 1.7068625689);
    vec3 lms1 = pow(cone2lms * lin1.rgb, vec3(1.0 / 3.0));
    vec3 lms2 = pow(cone2lms * lin2.rgb, vec3(1.0 / 3.0));
    vec3 lms = mix(lms1, lms2, a);
    return vec4(lms2cone * (lms * lms * lms), mix(lin1.a, lin2.a, a));
}

layout(binding = 1) uniform sampler2D atlas;

vec2 render(vec2 off, float blur) {
    if(roundness == 0.0) {
        return vec2(1, 0);
    }

    vec2 uv = uv + off;

    // text rendering
    if(roundness < 0.0) {
        float bold = -(1.0 + roundness);

        vec2 p = ((uv / 1.125 * 0.5 + 0.5) * vec2(tex_coord.zw) + vec2(tex_coord.xy)) / vec2(textureSize(atlas, 0));

        float r = 2.0 * (0.5 - texture(atlas, p).r * (bold * 0.5 + 1.0));

        vec3 pd = vec3(length(dFdx(p)), length(dFdy(p)), 0.0) * 0.75;

        vec2 pp = vec2(texture(atlas, p + pd.xz).r, texture(atlas, p + pd.zy).r);
        vec2 nn = vec2(texture(atlas, p - pd.xz).r, texture(atlas, p - pd.zy).r);

        float d = length(pp - nn) * (bold * 0.5 + 1.0);
        float edge = clamp(1.0 - r / mix(d, 1.0, blur), 0.0, 1.0);
        float strk = (stroke_width < 0.001 && blur >= 0.0) ? 0.0 : clamp((r + stroke_width * 0.5 * (1.0 + bold)) / mix(d, 1.0, stroke_blur + blur), 0.0, 1.0);

        return vec2(edge, strk);
    }
    // primitive rendering
    else {
        float r = elongated_rrect(uv * scale, roundness, scale - 1.0);
        float d = length(vec2(dFdx(r), dFdy(r)));
        float edge = clamp(1.0 - roundness - r / mix(d, 0.5, blur), 0.0, 1.0);
        float strk = (stroke_width < 0.001 && blur >= 0.0) ? 0.0 : clamp((r + stroke_width * 1.05) / mix(d, 0.5, stroke_blur + blur), 0.0, 1.0);
        return vec2(edge, strk);
    }
}

void main() {
    float pblur = max(0.0, blur);
    float grad = smoothstep(-1.0, 1.0, dot(uv, gradient_dir));
    if(dot(gradient_dir, gradient_dir) > 0.01) {
        out_color = oklab_mix(color, gradient_color, grad);
    } else {
        out_color = color;
    }
    vec2 es = render(vec2(0), pblur);
    // text rendering
    if(blur <= 0.0) {
        // supersampling for sharp text
        if(roundness < 0.0) {
            float dx = length(dFdx(uv)) / 4.0;
            vec2 xl = render(vec2(-dx, 0), pblur);
            vec2 xr = render(vec2(dx, 0), pblur);
            float dy = length(dFdy(uv)) / 4.0;
            vec2 yl = render(vec2(0, -dy), pblur);
            vec2 yr = render(vec2(0, dy), pblur);
            es = (es + xl + xr + yl + yr) / 5.0;
        }
        if(blur < 0.0) {
            vec2 glow = render(vec2(0), -blur);
            es.y = mix(glow.y, 1.0, es.y);
            es.x = mix(1.0, glow.x, es.y);
        }
    }
    out_color = oklab_mix(out_color, vec4(stroke_color.rgb, out_color.a), es.y);
    out_color = vec4(lrgb2srgb(out_color.rgb), out_color.a * es.x);
    // textured primitive
    if(roundness >= 0.0 && tex_coord.x != ~0u) {
        uvec2 p = uvec2((uv * 0.5 + 0.5) * vec2(tex_coord.zw)) + tex_coord.xy;
        out_color *= texture(atlas, vec2(p) / textureSize(atlas, 0).xy);
    }
    if(out_color.a < 0.001) {
        discard;
    }
}
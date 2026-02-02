#version 450

layout(location = 0) in uint i_position;
layout(location = 1) in uint i_scale;
layout(location = 2) in uint i_color;
layout(location = 3) in float i_roundness; // negative for text
layout(location = 4) in float i_rotation;
layout(location = 5) in float i_stroke_width;
layout(location = 6) in uint i_stroke_color;
layout(location = 7) in uvec2 i_tex_coord; // packed whxy
layout(location = 8) in float i_blur; // negative = glow
layout(location = 9) in float i_stroke_blur;
layout(location = 10) in uint i_gradient_color;
layout(location = 11) in float i_gradient_dir; // MAX = no gradient
layout(location = 12) in float i_superellipse;

layout(location = 0) out vec2 uv;
layout(location = 1) flat out vec4 color;
layout(location = 2) flat out float roundness;
layout(location = 3) flat out vec4 stroke_color;
layout(location = 4) flat out float stroke_width;
layout(location = 5) flat out vec2 scale;
layout(location = 6) flat out uvec4 tex_coord;
layout(location = 7) flat out float blur;
layout(location = 8) flat out float stroke_blur;
layout(location = 9) flat out vec4 gradient_color;
layout(location = 10) flat out vec2 gradient_dir;
layout(location = 11) flat out float superellipse;

layout(std140, binding = 0) uniform Uniform {
    vec2 res;
};

vec3 srgb2lrgb(vec3 srgb) {
    return mix(srgb / 12.92, pow((srgb + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), srgb));
}

void main() {
    uv = vec2(uvec2(gl_VertexIndex % 2, gl_VertexIndex / 2));
    // heuristic sizing to avoid blur going out of bounds
    uv = (uv * 2.0 - 1.0) * (1.05 + max(0.17 * abs(i_blur), -(i_roundness + 1.0) * 0.1));

    scale = unpackUnorm2x16(i_scale);
    vec2 scaled_uv = scale * uv;
    vec2 new_uv = scaled_uv * cos(i_rotation) + vec2(-scaled_uv.y, scaled_uv.x) * res.yx / res * sin(i_rotation);
    vec2 pos = unpackUnorm2x16(i_position);

    gl_Position = vec4(pos * 2.0 - 1.0 + new_uv * 2.0, 0, 1);

    color = unpackUnorm4x8(i_color);
    color.rgb = srgb2lrgb(color.rgb);
    roundness = i_roundness;
    stroke_width = i_stroke_width;
    stroke_color = unpackUnorm4x8(i_stroke_color);
    stroke_color.rgb = srgb2lrgb(stroke_color.rgb);
    scale *= res;
    scale /= min(scale.x, scale.y);
    if(i_tex_coord.x > 0) {
        tex_coord = uvec4(i_tex_coord.y >> 16, i_tex_coord.y, i_tex_coord.x >> 16, i_tex_coord.x) & uvec4(0xFFFF);
    } else {
        tex_coord = uvec4(~0u);
    }
    blur = i_blur;
    stroke_blur = i_stroke_blur;
    gradient_color = unpackUnorm4x8(i_gradient_color);
    gradient_color.rgb = srgb2lrgb(gradient_color.rgb);
    bool use_grad = abs(i_gradient_dir) < 1e9;
    gradient_color = use_grad ? gradient_color : color;
    gradient_dir = use_grad ? vec2(cos(i_gradient_dir), sin(i_gradient_dir)) : vec2(0);
    superellipse = i_superellipse;
}
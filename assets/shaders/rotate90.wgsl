@group(0) @binding(0) var atlas: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var texture: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(2) var<uniform> location: vec2u;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3u)  {
    let tex = textureLoad(texture, global_id.xy);
    textureStore(atlas, global_id.yx + location.xy, tex);
}
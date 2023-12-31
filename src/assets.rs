pub fn get_shader(name: &str) -> wgpu::ShaderModuleDescriptor {
    let path = format!("assets/shaders/{}", name);
    let shader_content = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("Failed to open shader file {}: {}", path, err));

    wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(shader_content)),
    }
}

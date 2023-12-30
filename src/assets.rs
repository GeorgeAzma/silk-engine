pub fn get<P: AsRef<std::path::Path>>(path: P) -> String {
    let assets_path = "assets";
    let full_path = path.as_ref().to_string_lossy();
    format!("{}/{}", assets_path, full_path)
}

pub fn get_shader(name: &str) -> wgpu::ShaderModuleDescriptor {
    let path = format!("{}/{}", get("shaders"), name);
    let shader_content = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("Failed to open shader file {}: {}", path, err));

    wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(shader_content)),
    }
}

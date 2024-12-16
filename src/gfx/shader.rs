use super::vulkan::pipeline::PipelineStageInfo;

use crate::*;
use gfx::DSLBinding;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref INIT_CACHE_PATH: () = {
        std::fs::create_dir_all("res/cache/shaders").unwrap_or_default();
    };
}

fn shader_path(name: &str) -> String {
    format!("res/shaders/{name}.wgsl")
}

fn shader_cache_path(name: &str) -> String {
    format!("res/cache/shaders/{name}.spv")
}

/*
Input:
- pipeline state
- shader files

Output:
- pipeline (cached)
*/

pub struct Shader {
    spirv: Vec<u32>,
    ir_module: naga::Module,
    entry_names: Vec<String>,
}

impl Shader {
    pub fn new(name: &str) -> Self {
        // TODO: save/load reflection (using naga's serde serialize feature) (only if bottlenecked)
        // parse wgsl
        let source = std::fs::read_to_string(shader_path(name)).unwrap();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| {
            fatal!("WGSL Error:\n{}", e.emit_to_string(&source));
        });
        let mut entry_names = Vec::new();
        for entry in module.entry_points.iter() {
            let mut name = entry.name.clone();
            name.push('\0');
            entry_names.push(name);
        }

        // read spirv cache
        let spirv = if let Ok(spirv) = std::fs::read(shader_cache_path(name)) {
            log!("shader cache loaded: \"{name}.spv\"");
            util::cast_slice_to(&spirv).to_owned()
        } else {
            log!("shader loaded: \"{name}.wgsl\"");
            // validate wgsl
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .expect("validation failed");

            // generate spirv
            let mut spirv = vec![];
            let opts = naga::back::spv::Options {
                lang_version: (1, 3),
                ..Default::default()
            };
            let mut writer = naga::back::spv::Writer::new(&opts).unwrap();
            writer
                .write(&module, &info, None, &None, &mut spirv)
                .unwrap();

            // write spirv cache
            #[cfg(not(debug_assertions))]
            *INIT_CACHE_PATH;
            #[cfg(not(debug_assertions))]
            std::fs::write(&shader_cache_path(name), util::cast_slice(&spirv)).unwrap_or_default();

            spirv
        };

        Self {
            spirv,
            ir_module: module,
            entry_names,
        }
    }

    pub fn create_module(&self) -> vk::ShaderModule {
        unsafe {
            DEVICE
                .create_shader_module(
                    &vk::ShaderModuleCreateInfo::default().code(&self.spirv),
                    None,
                )
                .unwrap()
        }
    }

    pub fn workgroup_size(&self) -> [u32; 3] {
        self.ir_module.entry_points[0].workgroup_size
    }

    pub fn get_dsl_bindings(&self) -> Vec<Vec<DSLBinding>> {
        let mut bindings: HashMap<u32, Vec<DSLBinding>> = HashMap::new();
        let mut resource_access_stages: HashMap<u32, vk::ShaderStageFlags> = HashMap::new();
        for entry in self.ir_module.entry_points.iter() {
            for (expr_hnd, expr) in entry.function.expressions.iter() {
                if matches!(
                    expr,
                    naga::Expression::Access { .. }
                        | naga::Expression::AccessIndex { .. }
                        | naga::Expression::FunctionArgument(_)
                        | naga::Expression::GlobalVariable(_)
                        | naga::Expression::LocalVariable(_)
                ) {
                    if let Some(gvar_hnd) = entry.function.originating_global(expr_hnd) {
                        let gvar = &self.ir_module.global_variables[gvar_hnd];
                        if let Some(naga::ResourceBinding { group, binding }) = gvar.binding {
                            let resource_key = group << 16 | binding;
                            let stage = stage_to_vk(&entry.stage);
                            resource_access_stages
                                .entry(resource_key)
                                .and_modify(|stages| *stages |= stage)
                                .or_insert(stage);
                        }
                    }
                }
            }
        }
        for (_, gvar) in self.ir_module.global_variables.iter() {
            if let Some(naga::ResourceBinding { group, binding }) = gvar.binding {
                let resource_key = group << 16 | binding;
                let array_size = match self.ir_module.types[gvar.ty].inner.clone() {
                    naga::TypeInner::Array {
                        size, stride: _, ..
                    }
                    | naga::TypeInner::BindingArray { size, .. } => {
                        if let naga::ArraySize::Constant(size) = size {
                            size.get()
                        } else {
                            1
                        }
                    }
                    _ => 1,
                };
                let desc_type = match (gvar.space, self.ir_module.types[gvar.ty].inner.clone()) {
                    (naga::AddressSpace::Handle, naga::TypeInner::Sampler { .. }) => {
                        vk::DescriptorType::SAMPLER
                    }
                    (naga::AddressSpace::Handle, naga::TypeInner::Image { .. }) => {
                        vk::DescriptorType::SAMPLED_IMAGE
                    }
                    (naga::AddressSpace::Storage { .. }, naga::TypeInner::Image { .. }) => {
                        vk::DescriptorType::STORAGE_IMAGE
                    }
                    (naga::AddressSpace::Uniform, _) => vk::DescriptorType::UNIFORM_BUFFER,
                    (naga::AddressSpace::Storage { .. }, _) => vk::DescriptorType::STORAGE_BUFFER,
                    (_, _) => vk::DescriptorType::from_raw(-1),
                };
                let binding = DSLBinding {
                    binding,
                    descriptor_type: desc_type,
                    descriptor_count: array_size,
                    stage_flags: *resource_access_stages
                        .get(&resource_key)
                        .unwrap_or(&vk::ShaderStageFlags::empty()),
                };
                bindings.entry(group).or_default().push(binding);
            }
        }
        let bindings = bindings.into_iter().collect::<Vec<_>>();
        let max_group = bindings
            .iter()
            .map(|(group, _)| group)
            .cloned()
            .max()
            .unwrap_or(0) as usize;
        let mut bindings_vec = vec![Default::default(); max_group + 1];
        for (group, binding) in bindings {
            bindings_vec[group as usize] = binding;
        }
        bindings_vec
    }

    pub fn create_dsls(&self) -> Vec<vk::DescriptorSetLayout> {
        self.get_dsl_bindings()
            .iter()
            .map(|dslb| DSL_MANAGER.write().unwrap().get(dslb))
            .collect()
    }

    /// Arguments:
    /// - bindings: `(instanced, resource_locations)`
    ///   - if bindings is empty, resources are put in single binding(0) with location automatically determined
    ///   - if one of the binding's resource_locations is empty, unlocated resources are put in that binding automatically
    ///
    /// Returns:
    /// - `(attrib_descs, binding_descs)` binding_descs[i] corresponds to attrib_descs with binding=i
    pub fn get_vert_layout(
        &self,
        bindings: &[(bool, Vec<u32>)],
    ) -> (
        Vec<vk::VertexInputBindingDescription>,
        Vec<vk::VertexInputAttributeDescription>,
    ) {
        let default_binding = vec![(false, vec![])];
        let bindings = if bindings.is_empty() {
            &default_binding
        } else {
            bindings
        };
        let auto_location_binding = bindings
            .iter()
            .position(|(_, locs)| locs.is_empty())
            .map(|i| i as u32);
        let mut location_binding = HashMap::new();
        bindings.iter().enumerate().for_each(|(i, (_, locs))| {
            locs.iter().for_each(|l| {
                location_binding.insert(*l, i as u32);
            });
        });
        fn calc_vert_attrib_descs(
            binding: Option<&naga::Binding>,
            ty: &naga::TypeInner,
            module: &naga::Module,
            binding_offset: &mut [u32],
            vert_attrib_descs: &mut Vec<vk::VertexInputAttributeDescription>,
            location_binding: &HashMap<u32, u32>,
            name: &str,
            auto_location_binding: Option<u32>,
        ) {
            let mut push_attrib = |ty: &naga::TypeInner, location: u32| {
                let format = type_to_vk(ty);
                let binding = location_binding
                    .get(&location)
                    .cloned()
                    .or(auto_location_binding)
                    .unwrap_or_else(|| panic!("unused @location({location}) {name}"));
                let offset = &mut binding_offset[binding as usize];
                vert_attrib_descs.push(vk::VertexInputAttributeDescription {
                    location,
                    binding,
                    format,
                    offset: *offset,
                });
                *offset += format_size(format);
            };
            match binding {
                Some(binding) => match binding {
                    naga::Binding::Location { location, .. } => {
                        push_attrib(ty, *location);
                    }
                    naga::Binding::BuiltIn(_) => {}
                },
                None => {
                    if let naga::TypeInner::Struct { members, .. } = ty {
                        for member in members {
                            calc_vert_attrib_descs(
                                member.binding.as_ref(),
                                &module.types[member.ty].inner,
                                module,
                                binding_offset,
                                vert_attrib_descs,
                                location_binding,
                                &member.name.clone().unwrap_or_default(),
                                auto_location_binding,
                            );
                        }
                    }
                }
            }
        }

        let mut binding_offset = vec![0; bindings.len()];
        let mut vert_attrib_descs = vec![];
        for entry_point in self.ir_module.entry_points.iter() {
            if entry_point.stage != naga::ShaderStage::Vertex {
                continue;
            }
            for arg in entry_point.function.arguments.iter() {
                calc_vert_attrib_descs(
                    arg.binding.as_ref(),
                    &self.ir_module.types[arg.ty].inner,
                    &self.ir_module,
                    &mut binding_offset,
                    &mut vert_attrib_descs,
                    &location_binding,
                    &arg.name.clone().unwrap_or_default(),
                    auto_location_binding,
                );
            }
        }

        (
            bindings
                .iter()
                .enumerate()
                .filter_map(|(i, binding)| {
                    if binding.1.is_empty() {
                        None
                    } else {
                        Some(
                            vk::VertexInputBindingDescription::default()
                                .stride(binding_offset[i])
                                .binding(i as u32)
                                .input_rate(if binding.0 {
                                    vk::VertexInputRate::INSTANCE
                                } else {
                                    vk::VertexInputRate::VERTEX
                                }),
                        )
                    }
                })
                .collect(),
            vert_attrib_descs,
        )
    }

    /// get_vert_layout with single binding
    pub fn get_vert_layout_binding(
        &self,
        instanced: bool,
    ) -> (
        vk::VertexInputBindingDescription,
        Vec<vk::VertexInputAttributeDescription>,
    ) {
        let (binding_descs, attrib_descs) = self.get_vert_layout(&[(instanced, vec![])]);
        (binding_descs[0], attrib_descs)
    }

    pub fn get_pipeline_stages(&self, module: vk::ShaderModule) -> Vec<PipelineStageInfo> {
        self.entry_names
            .iter()
            .zip(self.ir_module.entry_points.iter().map(|ep| &ep.stage))
            .map(|(entry_name, stage)| PipelineStageInfo {
                stage: stage_to_vk(stage),
                module,
                name: entry_name.clone(),
                ..Default::default()
            })
            .collect()
    }
}

fn vec_size_uint(size: &naga::VectorSize) -> u32 {
    match size {
        naga::VectorSize::Bi => 2,
        naga::VectorSize::Tri => 3,
        naga::VectorSize::Quad => 4,
    }
}

fn type_to_vk(ty: &naga::TypeInner) -> vk::Format {
    let type_info = |scalar: &naga::Scalar, size: Option<&naga::VectorSize>| {
        use naga::ScalarKind::*;
        match (scalar.kind, size.map_or(1, vec_size_uint)) {
            (Sint, 1) => vk::Format::R32_SINT,
            (Uint, 1) => vk::Format::R32_UINT,
            (Float, 1) => vk::Format::R32_SFLOAT,
            (Bool, 1) => vk::Format::R32_UINT,

            (Sint, 2) => vk::Format::R32G32_SINT,
            (Uint, 2) => vk::Format::R32G32_UINT,
            (Float, 2) => vk::Format::R32G32_SFLOAT,
            (Bool, 2) => vk::Format::R32G32_UINT,

            (Sint, 3) => vk::Format::R32G32B32_SINT,
            (Uint, 3) => vk::Format::R32G32B32_UINT,
            (Float, 3) => vk::Format::R32G32B32_SFLOAT,
            (Bool, 3) => vk::Format::R32G32B32_UINT,

            (Sint, 4) => vk::Format::R32G32B32A32_SINT,
            (Uint, 4) => vk::Format::R32G32B32A32_UINT,
            (Float, 4) => vk::Format::R32G32B32A32_SFLOAT,
            (Bool, 4) => vk::Format::R32G32B32A32_UINT,
            _ => panic!(),
        }
    };
    match ty {
        naga::TypeInner::Scalar(scalar) => type_info(scalar, None),
        naga::TypeInner::Vector { size, scalar } => type_info(scalar, Some(size)),
        _ => panic!(),
    }
}

fn stage_to_vk(stage: &naga::ShaderStage) -> vk::ShaderStageFlags {
    match stage {
        naga::ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
        naga::ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
        naga::ShaderStage::Compute => vk::ShaderStageFlags::COMPUTE,
    }
}

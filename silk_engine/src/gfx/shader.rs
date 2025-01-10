use std::collections::HashMap;

use super::{
    alloc_callbacks,
    vulkan::{pipeline::PipelineStageInfo, DSLBinding},
};
use crate::{format_size, gpu, log, RES_PATH};
use ash::vk;
use naga::Module;

fn shader_path(name: &str) -> String {
    format!("{RES_PATH}/shaders/{name}.wgsl")
}

fn shader_cache_path(name: &str) -> String {
    format!("{RES_PATH}/cache/shaders/{name}.spv")
}

pub struct Shader {
    spirv: Vec<u32>,
    ir_module: naga::Module,
    dsl_infos: Vec<Vec<DSLBinding>>, // [group, binding]
}

impl Shader {
    pub fn new(name: &str) -> Self {
        // TODO: save/load reflection (using naga's serde serialize feature) (only if bottlenecked)
        let source = std::fs::read_to_string(shader_path(name)).unwrap();
        let ir_module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| {
            panic!("WGSL {}", e.emit_to_string(&source));
        });

        // read spirv cache
        let spirv = if let Ok(spirv) = std::fs::read(shader_cache_path(name)) {
            log!("Shader cache loaded: \"{name}.spv\"");
            crate::util::as_slice(&spirv[..]).to_owned()
        } else {
            log!("Shader loaded: \"{name}.wgsl\"");
            // validate wgsl
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&ir_module)
            .expect("validation failed");

            // generate spirv
            let mut spirv = vec![];
            let opts = naga::back::spv::Options {
                lang_version: (1, 3),
                ..Default::default()
            };
            let mut writer = naga::back::spv::Writer::new(&opts).unwrap();
            writer
                .write(&ir_module, &info, None, &None, &mut spirv)
                .unwrap();

            // write spirv cache
            #[cfg(not(debug_assertions))]
            *crate::INIT_CACHE_PATH;
            #[cfg(not(debug_assertions))]
            std::fs::write(&shader_cache_path(name), crate::util::as_slice(&spirv[..])).unwrap();

            spirv
        };

        fn get_dsl_infos(ir_module: &Module) -> Vec<Vec<DSLBinding>> {
            let mut bindings: HashMap<u32, Vec<DSLBinding>> = HashMap::new();
            let mut resource_access_stages: HashMap<u32, vk::ShaderStageFlags> = HashMap::new();
            for entry in ir_module.entry_points.iter() {
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
                            let gvar = &ir_module.global_variables[gvar_hnd];
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
            for (_, gvar) in ir_module.global_variables.iter() {
                if let Some(naga::ResourceBinding { group, binding }) = gvar.binding {
                    let resource_key = group << 16 | binding;
                    let array_size = match ir_module.types[gvar.ty].inner.clone() {
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
                    let desc_type = match (gvar.space, ir_module.types[gvar.ty].inner.clone()) {
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
                        (naga::AddressSpace::Storage { .. }, _) => {
                            vk::DescriptorType::STORAGE_BUFFER
                        }
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
        let dsl_infos = get_dsl_infos(&ir_module);
        Self {
            spirv,
            ir_module,
            dsl_infos,
        }
    }

    pub fn dsl_infos(&self) -> &[Vec<DSLBinding>] {
        &self.dsl_infos
    }

    pub fn create_module(&self) -> vk::ShaderModule {
        unsafe {
            gpu()
                .create_shader_module(
                    &vk::ShaderModuleCreateInfo::default().code(&self.spirv),
                    alloc_callbacks(),
                )
                .unwrap()
        }
    }

    pub fn workgroup_size(&self) -> [u32; 3] {
        self.ir_module.entry_points[0].workgroup_size
    }

    /// Arguments:
    /// - bindings: `(instanced, resource_locations)`
    ///   - if bindings is empty, resources are put in single binding(0) with location automatically determined
    ///   - if one of the binding's resource_locations is empty, unlocated resources are put in that binding automatically
    ///
    /// Returns:
    /// - `(attrib_descs, binding_descs)` `binding_descs[i]` corresponds to `attrib_descs` with `binding=i`
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
            if vert_attrib_descs.is_empty() {
                vec![]
            } else {
                bindings
                    .iter()
                    .enumerate()
                    .map(|(i, &(instanced, _))| {
                        vk::VertexInputBindingDescription::default()
                            .stride(binding_offset[i])
                            .binding(i as u32)
                            .input_rate(if instanced {
                                vk::VertexInputRate::INSTANCE
                            } else {
                                vk::VertexInputRate::VERTEX
                            })
                    })
                    .collect()
            },
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
        self.ir_module
            .entry_points
            .iter()
            .map(|ep| {
                let mut name = ep.name.clone();
                if !name.ends_with('\0') {
                    name.push('\0');
                }
                PipelineStageInfo {
                    stage: stage_to_vk(&ep.stage),
                    module,
                    name,
                    ..Default::default()
                }
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

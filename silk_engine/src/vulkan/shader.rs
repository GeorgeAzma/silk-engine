use std::{collections::BTreeMap, path::Path, sync::Arc, sync::OnceLock};

use ash::vk;
use rspirv_reflect::{
    DescriptorInfo, PushConstantInfo,
    rspirv::dr::Module,
    spirv::{self, StorageClass},
};
use shaderc::ShaderKind;

use crate::{
    prelude::ResultAny,
    util::{cast_slice, cast_vec},
    vulkan::device::Device,
};

struct CompiledShader {
    spirv: Vec<u32>,
    descriptor_sets: BTreeMap<u32, BTreeMap<u32, DescriptorInfo>>,
    workgroup_size: Option<(u32, u32, u32)>,
    push_constant_ranges: Option<PushConstantInfo>,
    kind: ShaderKind,
    stage: vk::ShaderStageFlags,
    module: Module,
}

impl CompiledShader {
    fn new(file: &str) -> ResultAny<Self> {
        let path = Path::new(file);
        let ext = path.extension().and_then(|os| os.to_str()).unwrap_or("");
        let name = path.file_stem().and_then(|os| os.to_str()).unwrap_or("");
        let cache = format!("res/cache/shaders/{name}.{ext}");
        let cache = Path::new(&cache);
        let kind = Self::ext_to_kind(ext)?;
        let stage = Self::kind_to_stage(kind)?;

        let cache_modified = cache.metadata().and_then(|cache| cache.modified());
        let shader_modified = path.metadata().and_then(|path| path.modified());
        let cache_outdated = match (cache_modified, shader_modified) {
            (Ok(cache), Ok(shader)) => cache <= shader,
            _ => true,
        };
        let use_cache = if cfg!(debug_assertions) {
            false
        } else {
            cache.exists() && !cache_outdated
        };

        let spirv = if use_cache {
            let spirv = std::fs::read(cache)?;
            cast_vec(spirv)
        } else {
            let compiler = shaderc::Compiler::new().unwrap();

            let is_hlsl = name.ends_with(".hlsl");
            let mut options = shaderc::CompileOptions::new().unwrap();
            options.set_optimization_level(shaderc::OptimizationLevel::Performance);
            options.set_source_language(if is_hlsl {
                shaderc::SourceLanguage::HLSL
            } else {
                shaderc::SourceLanguage::GLSL
            });
            #[cfg(debug_assertions)]
            options.set_generate_debug_info();

            let spirv_compilation = compiler.compile_into_spirv(
                &std::fs::read_to_string(file)
                    .map_err(|e| format!("Failed to read {}: {}", file, e))?,
                kind,
                file,
                "main",
                Some(&options),
            )?;

            let warnings = spirv_compilation.get_warning_messages();
            if !warnings.is_empty() {
                crate::warn!("{file}: {warnings}");
            }

            let spirv = spirv_compilation.as_binary();
            assert_eq!(Some(&0x07230203), spirv.first());
            std::fs::write(cache, cast_slice(spirv))?;

            crate::info!("compiled {file}");

            spirv.to_vec()
        };

        let reflect = rspirv_reflect::Reflection::new_from_spirv(cast_slice(&spirv))?;
        let descriptor_sets = reflect.get_descriptor_sets()?;
        let workgroup_size = reflect.get_compute_group_size();
        let push_constant_ranges = reflect.get_push_constant_range()?;
        let module = reflect.0;

        Ok(Self {
            spirv,
            descriptor_sets,
            workgroup_size,
            push_constant_ranges,
            kind,
            stage,
            module,
        })
    }

    fn ext_to_kind(ext: &str) -> ResultAny<ShaderKind> {
        let ext = ext.strip_prefix('.').unwrap_or(ext);
        Ok(match ext {
            "vertex" | "vert" | "vrt" | "vs" => ShaderKind::Vertex,
            "fragment" | "frag" | "frg" | "fs" => ShaderKind::Fragment,
            "compute" | "comp" | "cmp" | "cs" => ShaderKind::Compute,
            "geometry" | "geom" | "geo" | "gm" | "gs" => ShaderKind::Geometry,
            "tessellation_control" | "tess_control" | "tess_ctrl" | "tesc" | "tcs" => {
                ShaderKind::TessControl
            }
            "tesselation_evaluation" | "tess_evaluation" | "tess_eval" | "tese" | "tes" => {
                ShaderKind::TessEvaluation
            }
            "spirv_assembly" | "spirv_asm" | "spv_asm" | "spv" | "sprv" | "spirv" => {
                ShaderKind::SpirvAssembly
            }
            "ray_generation" | "ray_gen" | "raygen" | "rgen" | "rgn" | "rg" => {
                ShaderKind::RayGeneration
            }
            "any_hit" | "any" | "ah" | "rahit" => ShaderKind::AnyHit,
            "closest_hit" | "closest" | "ch" | "rchit" => ShaderKind::ClosestHit,
            "miss" | "rmiss" => ShaderKind::Miss,
            "intersection" | "inter" | "int" | "rint" => ShaderKind::Intersection,
            "callable" | "call" | "cal" | "rcall" => ShaderKind::Callable,
            "mesh" | "msh" | "ms" => ShaderKind::Mesh,
            "task" | "tsk" | "tk" => ShaderKind::Task,
            _ => return Err(format!("invalid shader extension: {ext}").into()),
        })
    }

    fn kind_to_stage(kind: ShaderKind) -> ResultAny<vk::ShaderStageFlags> {
        Ok(match kind {
            ShaderKind::Vertex | ShaderKind::DefaultVertex => vk::ShaderStageFlags::VERTEX,
            ShaderKind::Fragment | ShaderKind::DefaultFragment => vk::ShaderStageFlags::FRAGMENT,
            ShaderKind::Compute | ShaderKind::DefaultCompute => vk::ShaderStageFlags::COMPUTE,
            ShaderKind::Geometry | ShaderKind::DefaultGeometry => vk::ShaderStageFlags::GEOMETRY,
            ShaderKind::TessControl | ShaderKind::DefaultTessControl => {
                vk::ShaderStageFlags::TESSELLATION_CONTROL
            }
            ShaderKind::TessEvaluation | ShaderKind::DefaultTessEvaluation => {
                vk::ShaderStageFlags::TESSELLATION_EVALUATION
            }
            // ShaderKind::InferFromSource => vk::ShaderStageFlags::,
            // ShaderKind::SpirvAssembly => vk::ShaderStageFlags::VERTEX,
            ShaderKind::RayGeneration | ShaderKind::DefaultRayGeneration => {
                vk::ShaderStageFlags::RAYGEN_KHR
            }
            ShaderKind::AnyHit | ShaderKind::DefaultAnyHit => vk::ShaderStageFlags::ANY_HIT_KHR,
            ShaderKind::ClosestHit | ShaderKind::DefaultClosestHit => {
                vk::ShaderStageFlags::CLOSEST_HIT_KHR
            }
            ShaderKind::Miss | ShaderKind::DefaultMiss => vk::ShaderStageFlags::MISS_KHR,
            ShaderKind::Intersection | ShaderKind::DefaultIntersection => {
                vk::ShaderStageFlags::INTERSECTION_KHR
            }
            ShaderKind::Callable | ShaderKind::DefaultCallable => {
                vk::ShaderStageFlags::CALLABLE_KHR
            }
            ShaderKind::Task | ShaderKind::DefaultTask => vk::ShaderStageFlags::TASK_EXT,
            ShaderKind::Mesh | ShaderKind::DefaultMesh => vk::ShaderStageFlags::MESH_EXT,
            _ => return Err(format!("invalid shader kind: {kind:?}").into()),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VertexInput {
    pub(crate) location: u32,
    pub(crate) format: vk::Format,
    pub(crate) name: String,
}

pub(crate) struct Shader {
    shaders: Vec<(CompiledShader, vk::ShaderModule)>,
    set_bindings: OnceLock<BTreeMap<u32, BTreeMap<u32, vk::DescriptorSetLayoutBinding<'static>>>>,
    device: Arc<Device>,
}

impl Shader {
    pub(crate) fn new(files: &[&str], device: &Arc<Device>) -> ResultAny<Self> {
        let path = "res/shaders";
        let shaders = files
            .iter()
            .map(|file| {
                let compiled_shader = CompiledShader::new(&format!("{path}/{file}"))?;
                let module_info =
                    vk::ShaderModuleCreateInfo::default().code(&compiled_shader.spirv);
                let module = unsafe {
                    device
                        .device
                        .create_shader_module(&module_info, device.allocation_callbacks().as_ref())
                }?;
                Ok((compiled_shader, module))
            })
            .collect::<ResultAny<_>>()?;

        Ok(Self {
            shaders,
            set_bindings: OnceLock::new(),
            device: Arc::clone(device),
        })
    }

    pub(crate) fn pipeline_shader_stage_infos<'a>(
        &self,
        specialization_info: &'a vk::SpecializationInfo<'a>,
    ) -> Vec<vk::PipelineShaderStageCreateInfo<'a>> {
        self.shaders
            .iter()
            .map(|&(ref compiled_shader, module, ..)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(compiled_shader.stage)
                    .module(module)
                    .name(c"main")
                    .specialization_info(specialization_info)
            })
            .collect()
    }

    /// Returns { descriptor_set_idx: { binding_idx: binding } } reflected from all shader stages
    pub(crate) fn reflect_descriptor_set_layout_bindings(
        &self,
    ) -> ResultAny<BTreeMap<u32, BTreeMap<u32, vk::DescriptorSetLayoutBinding<'static>>>> {
        if let Some(set_bindings) = self.set_bindings.get() {
            return Ok(set_bindings.clone());
        }

        let mut set_bindings: BTreeMap<
            u32,
            BTreeMap<u32, vk::DescriptorSetLayoutBinding<'static>>,
        > = BTreeMap::new();

        for (compiled_shader, ..) in &self.shaders {
            let stage_flag = compiled_shader.stage;
            for (&set_idx, ds_bindings) in &compiled_shader.descriptor_sets {
                let bindings = set_bindings.entry(set_idx).or_default();
                for (&bind_idx, info) in ds_bindings {
                    bindings
                        .entry(bind_idx)
                        .and_modify(|binding| binding.stage_flags |= stage_flag)
                        .or_insert(
                            vk::DescriptorSetLayoutBinding::default()
                                .binding(bind_idx)
                                .descriptor_type(vk::DescriptorType::from_raw(info.ty.0 as i32))
                                .descriptor_count(match info.binding_count {
                                    rspirv_reflect::BindingCount::One => 1,
                                    rspirv_reflect::BindingCount::StaticSized(size) => size as u32,
                                    rspirv_reflect::BindingCount::Unbounded => 1,
                                })
                                .stage_flags(stage_flag),
                        );
                }
            }
        }
        self.set_bindings.set(set_bindings.clone()).unwrap();
        Ok(set_bindings)
    }

    /// Generates descriptor set layout from all shader stage reflections
    pub(crate) fn reflect_descriptor_set_layouts(
        &self,
    ) -> ResultAny<BTreeMap<u32, vk::DescriptorSetLayout>> {
        let set_bindings = self.reflect_descriptor_set_layout_bindings()?;
        let ds_layouts = set_bindings
            .into_iter()
            .map(|(set_idx, bindings)| {
                let layout_bindings: Vec<vk::DescriptorSetLayoutBinding<'static>> =
                    bindings.into_values().collect();
                let layout = self.device().ds_layout(&layout_bindings)?;
                Ok((set_idx, layout))
            })
            .collect::<ResultAny<_>>()?;
        Ok(ds_layouts)
    }

    /// Returns vertex input sorted by location
    pub(crate) fn reflect_vertex_input(&self) -> ResultAny<Vec<VertexInput>> {
        let Some((compiled_vertex_shader, ..)) =
            self.shaders.iter().find(|(compiled_shader, ..)| {
                compiled_shader.stage.contains(vk::ShaderStageFlags::VERTEX)
            })
        else {
            return Ok(vec![]);
        };
        let vertex_module = &compiled_vertex_shader.module;

        let mut variables = std::collections::BTreeMap::new();
        for inst in &vertex_module.types_global_values {
            if inst.class.opcode == spirv::Op::Variable {
                let storage_class = inst.operands[0].unwrap_storage_class();
                let type_id = inst.result_type.unwrap();
                variables.insert(inst.result_id.unwrap(), (storage_class, type_id));
            }
        }

        let mut names = std::collections::BTreeMap::new();
        for inst in &vertex_module.debug_names {
            if inst.class.opcode == spirv::Op::Name {
                let target_id = inst.operands[0].unwrap_id_ref();
                if let Some(name_op) = inst.operands.get(1) {
                    let name = name_op.to_string().replace("\"", "");
                    names.insert(target_id, name);
                }
            }
        }

        let mut vertex_inputs = vec![];

        // let mut locations = BTreeMap::<u32, u32>::new();
        for inst in &vertex_module.annotations {
            if inst.class.opcode == spirv::Op::Decorate {
                let id = inst.operands[0].unwrap_id_ref();
                let decoration = inst.operands[1].unwrap_decoration();
                if decoration == spirv::Decoration::Location
                    && let Some(name) = names.get(&id)
                    && let Some(&(StorageClass::Input, type_id)) = variables.get(&id)
                {
                    let location = inst.operands[2].unwrap_literal_bit32();
                    let format = Self::resolve_format(type_id, vertex_module)?;
                    let vertex_input = VertexInput {
                        location,
                        format,
                        name: name.clone(),
                    };
                    vertex_inputs.push(vertex_input);
                }
            }
        }
        vertex_inputs.sort_by_key(|input| input.location);

        Ok(vertex_inputs)
    }

    fn resolve_format(type_id: u32, module: &Module) -> ResultAny<vk::Format> {
        let type_inst = module
            .types_global_values
            .iter()
            .find(|inst| inst.result_id == Some(type_id))
            .ok_or(format!("Type ID {} not found", type_id))?;

        match type_inst.class.opcode {
            spirv::Op::TypeVector => {
                let component_type_id = type_inst.operands[0].unwrap_id_ref();
                let component_count = type_inst.operands[1].unwrap_literal_bit32();
                let component_format = Self::resolve_format(component_type_id, module)?;
                // Map vector to format (assuming 32-bit components)
                match (component_format, component_count) {
                    (vk::Format::R32_SFLOAT, 1) => Ok(vk::Format::R32_SFLOAT),
                    (vk::Format::R32_SFLOAT, 2) => Ok(vk::Format::R32G32_SFLOAT),
                    (vk::Format::R32_SFLOAT, 3) => Ok(vk::Format::R32G32B32_SFLOAT),
                    (vk::Format::R32_SFLOAT, 4) => Ok(vk::Format::R32G32B32A32_SFLOAT),

                    (vk::Format::R16_SFLOAT, 1) => Ok(vk::Format::R16_SFLOAT),
                    (vk::Format::R16_SFLOAT, 2) => Ok(vk::Format::R16G16_SFLOAT),
                    (vk::Format::R16_SFLOAT, 3) => Ok(vk::Format::R16G16B16_SFLOAT),
                    (vk::Format::R16_SFLOAT, 4) => Ok(vk::Format::R16G16B16A16_SFLOAT),

                    (vk::Format::R32_SINT, 1) => Ok(vk::Format::R32_SINT),
                    (vk::Format::R32_SINT, 2) => Ok(vk::Format::R32G32_SINT),
                    (vk::Format::R32_SINT, 3) => Ok(vk::Format::R32G32B32_SINT),
                    (vk::Format::R32_SINT, 4) => Ok(vk::Format::R32G32B32A32_SINT),

                    (vk::Format::R16_SINT, 1) => Ok(vk::Format::R16_SINT),
                    (vk::Format::R16_SINT, 2) => Ok(vk::Format::R16G16_SINT),
                    (vk::Format::R16_SINT, 3) => Ok(vk::Format::R16G16B16_SINT),
                    (vk::Format::R16_SINT, 4) => Ok(vk::Format::R16G16B16A16_SINT),

                    (vk::Format::R8_SINT, 1) => Ok(vk::Format::R8_SINT),
                    (vk::Format::R8_SINT, 2) => Ok(vk::Format::R8G8_SINT),
                    (vk::Format::R8_SINT, 3) => Ok(vk::Format::R8G8B8_SINT),
                    (vk::Format::R8_SINT, 4) => Ok(vk::Format::R8G8B8A8_SINT),

                    (vk::Format::R32_UINT, 1) => Ok(vk::Format::R32_UINT),
                    (vk::Format::R32_UINT, 2) => Ok(vk::Format::R32G32_UINT),
                    (vk::Format::R32_UINT, 3) => Ok(vk::Format::R32G32B32_UINT),
                    (vk::Format::R32_UINT, 4) => Ok(vk::Format::R32G32B32A32_UINT),

                    (vk::Format::R16_UINT, 1) => Ok(vk::Format::R16_UINT),
                    (vk::Format::R16_UINT, 2) => Ok(vk::Format::R16G16_UINT),
                    (vk::Format::R16_UINT, 3) => Ok(vk::Format::R16G16B16_UINT),
                    (vk::Format::R16_UINT, 4) => Ok(vk::Format::R16G16B16A16_UINT),

                    (vk::Format::R8_UINT, 1) => Ok(vk::Format::R8_UINT),
                    (vk::Format::R8_UINT, 2) => Ok(vk::Format::R8G8_UINT),
                    (vk::Format::R8_UINT, 3) => Ok(vk::Format::R8G8B8_UINT),
                    (vk::Format::R8_UINT, 4) => Ok(vk::Format::R8G8B8A8_UINT),
                    _ => Err(format!(
                        "Unsupported vector format: {} components of {:?}",
                        component_count, component_format
                    )
                    .into()),
                }
            }
            spirv::Op::TypeFloat => {
                let width = type_inst.operands[0].unwrap_literal_bit32();
                match width {
                    32 => Ok(vk::Format::R32_SFLOAT),
                    16 => Ok(vk::Format::R16_SFLOAT),
                    _ => Err(format!("Unsupported float width: {}", width).into()),
                }
            }
            spirv::Op::TypeInt => {
                let width = type_inst.operands[0].unwrap_literal_bit32();
                let signed = type_inst.operands[1].unwrap_literal_bit32() != 0;
                match (width, signed) {
                    (32, true) => Ok(vk::Format::R32_SINT),
                    (16, true) => Ok(vk::Format::R16_SINT),
                    (8, true) => Ok(vk::Format::R8_SINT),
                    (32, false) => Ok(vk::Format::R32_UINT),
                    (16, false) => Ok(vk::Format::R16_UINT),
                    (8, false) => Ok(vk::Format::R8_UINT),
                    _ => Err(format!("Unsupported int width: {}", width).into()),
                }
            }
            spirv::Op::TypePointer => {
                let type_id = type_inst.operands[1].unwrap_id_ref();
                Self::resolve_format(type_id, module)
            }
            _ => Err(format!("Unsupported type opcode: {:?}", type_inst.class.opcode).into()),
        }
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        for &(ref _compiled_shader, module) in &self.shaders {
            unsafe {
                self.device
                    .device
                    .destroy_shader_module(module, self.device.allocation_callbacks().as_ref())
            };
        }
    }
}

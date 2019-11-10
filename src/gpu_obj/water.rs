use super::glsl_compiler;
use crate::model;
use wgpu::Device;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, TextureFormat};

pub struct WaterGpu {
    pipeline: wgpu::RenderPipeline,
}

impl WaterGpu {
    pub fn new(
        device: &Device,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        log::trace!("WaterGpu new");
        let pipeline = Self::create_pipeline(device, main_bind_group_layout, format).unwrap();;
        WaterGpu { pipeline }
    }

    pub fn create_pipeline(
        device: &Device,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) -> glsl_compiler::Result<wgpu::RenderPipeline> {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&main_bind_group_layout],
        });
        let vertex_size = std::mem::size_of::<model::Vertex>();
        // Create the render pipeline
        let vs_bytes = glsl_compiler::load("./src/shader/water.vert")?;
        let fs_bytes = glsl_compiler::load("./src/shader/water.frag")?;

        let vs_module = device.create_shader_module(&vs_bytes);
        let fs_module = device.create_shader_module(&fs_bytes);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
            color_states: &[wgpu::ColorStateDescriptor {
                format,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
        Ok(pipeline)
    }

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        log::trace!("WaterGpu render");
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &main_bind_group, &[]);
        rpass.draw(0..4, 0..1);
    }
}

impl super::trait_gpu::TraitGpu for WaterGpu {
    fn reload_shader(
        &mut self,
        device: &Device,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) {
        match Self::create_pipeline(device, main_bind_group_layout, format) {
            Ok(pipeline) => self.pipeline = pipeline,
            Err(x) => log::error!("{}", x),
        };
    }
}

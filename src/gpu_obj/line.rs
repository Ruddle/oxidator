use super::glsl_compiler;
use crate::model;
use wgpu::Device;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, TextureFormat};

pub struct LineGpu {
    instance_buf: wgpu::Buffer,
    instance_count: u32,
    pipeline: wgpu::RenderPipeline,
}

impl LineGpu {
    pub fn new(
        device: &Device,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        log::trace!("LineGpu new");

        let positions: Vec<f32> = Vec::new();

        let instance_buf = device
            .create_buffer_mapped(
                positions.len(),
                wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&positions);

        let pipeline = Self::create_pipeline(device, main_bind_group_layout, format).unwrap();;

        LineGpu {
            instance_buf,
            instance_count: 0,
            pipeline,
        }
    }

    pub fn create_pipeline(
        device: &Device,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) -> glsl_compiler::Result<wgpu::RenderPipeline> {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&main_bind_group_layout],
        });
        // Create the render pipeline
        let vs_bytes = glsl_compiler::load("./src/shader/line.vert")?;
        let fs_bytes = glsl_compiler::load("./src/shader/line.frag")?;

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
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
            color_states: &[wgpu::ColorStateDescriptor {
                format: format,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: (4 * (2 + 2 + 2)) as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 4 * 2,
                        shader_location: 1,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float,
                        offset: 4 * 4,
                        shader_location: 2,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float,
                        offset: 4 * 5,
                        shader_location: 3,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
        Ok(pipeline)
    }

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        log::trace!("LineGpu render");
        if self.instance_count > 0 {
            rpass.set_pipeline(&self.pipeline);
            rpass.set_vertex_buffers(0, &[(&self.instance_buf, 0)]);
            rpass.set_bind_group(0, main_bind_group, &[]);
            rpass.draw(0..4, 0..self.instance_count as u32);
        }
    }

    pub fn update_instance(&mut self, instance_attr: &[f32], device: &wgpu::Device) {
        log::trace!("LineGpu update_instance");
        let temp_buf = device
            .create_buffer_mapped(instance_attr.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(instance_attr);

        std::mem::replace(&mut self.instance_buf, temp_buf);
        self.instance_count = instance_attr.len() as u32 / 6;
    }
}

impl super::trait_gpu::TraitGpu for LineGpu {
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

use super::glsl_compiler;
use crate::model;
use wgpu::Device;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, TextureFormat};

pub struct ModelGpu {
    pub instance_attr_cpu_buf: Vec<f32>,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: usize,
    instance_buf: wgpu::Buffer,
    instance_count: u32,
    pipeline: wgpu::RenderPipeline,
}

impl ModelGpu {
    pub fn new(
        triangle_list: &model::TriangleList,
        device: &Device,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        log::trace!("ModelGpu new");
        // Create the vertex and index buffers
        let model::TriangleList {
            vertex_data,
            index_data,
        } = triangle_list;
        let vertex_buf = device
            .create_buffer_mapped(vertex_data.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&vertex_data);

        let index_buf = device
            .create_buffer_mapped(index_data.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&index_data);

        let positions: Vec<f32> = Vec::new();

        let instance_buf = device
            .create_buffer_mapped(
                positions.len(),
                wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&positions);

        let pipeline = Self::create_pipeline(device, main_bind_group_layout, format).unwrap();;

        ModelGpu {
            instance_attr_cpu_buf: Vec::new(),
            vertex_buf,
            index_buf,
            index_count: index_data.len(),
            instance_buf,
            instance_count: positions.len() as u32 / 3,
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
        let vertex_size = std::mem::size_of::<model::Vertex>();
        // Create the render pipeline
        let vs_bytes = glsl_compiler::load("./src/shader/cube_instanced.vert").unwrap();
        let fs_bytes = glsl_compiler::load("./src/shader/cube_instanced.frag").unwrap();

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
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
                wgpu::ColorStateDescriptor {
                    format: wgpu::TextureFormat::Rgba32Float,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALPHA,
                },
            ],
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
            vertex_buffers: &[
                wgpu::VertexBufferDescriptor {
                    stride: vertex_size as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float4,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float2,
                            offset: 4 * 4,
                            shader_location: 1,
                        },
                    ],
                },
                wgpu::VertexBufferDescriptor {
                    stride: (4 * 18) as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float4,
                            offset: 0,
                            shader_location: 2,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float4,
                            offset: 4 * 4,
                            shader_location: 3,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float4,
                            offset: 4 * 8,
                            shader_location: 4,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float4,
                            offset: 4 * 12,
                            shader_location: 5,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float,
                            offset: 4 * 16,
                            shader_location: 6,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float,
                            offset: 4 * 17,
                            shader_location: 7,
                        },
                    ],
                },
            ],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
        Ok(pipeline)
    }

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        log::trace!("ModelGpu render");
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, main_bind_group, &[]);
        rpass.set_index_buffer(&self.index_buf, 0);
        rpass.set_vertex_buffers(0, &[(&self.vertex_buf, 0), (&self.instance_buf, 0)]);
        rpass.draw_indexed(0..self.index_count as u32, 0, 0..self.instance_count as u32);
    }

    pub fn update_instance_dirty(&mut self, instance_attr: &[f32], device: &wgpu::Device) {
        log::trace!("ModelGpu update_instance");
        let temp_buf = device
            .create_buffer_mapped(instance_attr.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(instance_attr);

        std::mem::replace(&mut self.instance_buf, temp_buf);
        self.instance_count = instance_attr.len() as u32 / 18;
    }

    pub fn update_instance_dirty_own_buffer(&mut self, device: &wgpu::Device) {
        log::trace!("ModelGpu update_instance");
        let temp_buf = device
            .create_buffer_mapped(self.instance_attr_cpu_buf.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&self.instance_attr_cpu_buf);

        std::mem::replace(&mut self.instance_buf, temp_buf);
        self.instance_count = self.instance_attr_cpu_buf.len() as u32 / 18;
    }

    pub fn update_instance(
        &mut self,
        instance_attr: &[f32],
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        log::trace!("ModelGpu update_instance");

        for (i, chunk) in instance_attr.chunks(1800).enumerate() {
            let temp_buf = device
                .create_buffer_mapped(
                    chunk.len(),
                    wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC,
                )
                .fill_from_slice(chunk);

            encoder.copy_buffer_to_buffer(
                &temp_buf,
                0,
                &self.instance_buf,
                i as u64 * 1800_u64,
                chunk.len() as u64 * 4,
            );
        }
        // let temp_buf = device
        //     .create_buffer_mapped(
        //         instance_attr.len(),
        //         wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC,
        //     )
        //     .fill_from_slice(instance_attr);

        // encoder.copy_buffer_to_buffer(
        //     &temp_buf,
        //     0,
        //     &self.instance_buf,
        //     0,
        //     instance_attr.len() as u64 * 4,
        // );

        self.instance_count = instance_attr.len() as u32 / 18;
    }
}

impl super::trait_gpu::TraitGpu for ModelGpu {
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

use crate::cube;
use crate::shader;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, RenderPipeline, TextureFormat};
use wgpu::{CommandEncoder, Device};

pub struct CubeGpu {
    cube_vertex_buf: wgpu::Buffer,
    cube_index_buf: wgpu::Buffer,
    cube_index_count: usize,
    //
    //    bind_group: wgpu::BindGroup,
    //    uniform_buf: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl CubeGpu {
    pub fn new(
        device: &Device,
        init_encoder: &mut CommandEncoder,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        // Create the vertex and index buffers
        let vertex_size = std::mem::size_of::<cube::Vertex>();
        let (vertex_data, cube_index_data) = cube::create_vertices();
        let cube_vertex_buf = device
            .create_buffer_mapped(vertex_data.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&vertex_data);

        let cube_index_buf = device
            .create_buffer_mapped(cube_index_data.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&cube_index_data);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&main_bind_group_layout],
        });

        // Create the render pipeline
        let vs_bytes = shader::load_glsl(
            include_str!("shader/cube_instanced.vert"),
            shader::ShaderStage::Vertex,
        );
        let fs_bytes = shader::load_glsl(
            include_str!("shader/cube_instanced.frag"),
            shader::ShaderStage::Fragment,
        );
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
            color_states: &[wgpu::ColorStateDescriptor {
                format: format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
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
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
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
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        CubeGpu {
            cube_vertex_buf,
            cube_index_buf,
            cube_index_count: cube_index_data.len(),
            pipeline,
        }
    }

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, main_bind_group, &[]);
        //        rpass.set_bind_group(1, &self.bind_group, &[]);

        rpass.set_index_buffer(&self.cube_index_buf, 0);
        rpass.set_vertex_buffers(0, &[(&self.cube_vertex_buf, 0)]);
        rpass.draw_indexed(0..self.cube_index_count as u32, 0, 0..10000);
    }
}

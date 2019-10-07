use crate::heightmap;
use crate::shader;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, RenderPipeline, TextureFormat};
use wgpu::{CommandEncoder, Device};
pub struct HeightmapGpu {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    height_vertex_buf: wgpu::Buffer,
    height_index_buf: wgpu::Buffer,
    height_index_count: usize,
}

impl HeightmapGpu {
    pub fn new(
        device: &Device,
        init_encoder: &mut CommandEncoder,
        format: TextureFormat,

        main_bind_group_layout: &BindGroupLayout,
        width_n: u32,
        height_n: u32,
    ) -> Self {
        // Create the texture
        let size = 16u32;
        let texels = crate::fake_texels::checker(size as usize);
        let texture_extent = wgpu::Extent3d {
            width: size,
            height: size,
            depth: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });
        let texture_view = texture.create_default_view();
        let temp_buf = device
            .create_buffer_mapped(texels.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&texels);
        init_encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                row_pitch: 4 * size,
                image_height: size,
            },
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
            texture_extent,
        );

        // Create other resources
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::Always,
        });

        //Map size
        let map_size = [
            width_n * heightmap::CHUNK_SIZE,
            height_n * heightmap::CHUNK_SIZE,
            width_n,
            height_n,
            heightmap::CHUNK_SIZE,
        ];

        let uniform_buf = device
            .create_buffer_mapped(5, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&map_size);

        // Create pipeline layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler,
                },
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &uniform_buf,
                        range: 0..12,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[main_bind_group_layout, &bind_group_layout],
        });

        // Create the render pipeline
        let vs_bytes = shader::load_glsl(
            include_str!("shader/heightmap.vert"),
            shader::ShaderStage::Vertex,
        );
        let fs_bytes = shader::load_glsl(
            include_str!("shader/heightmap.frag"),
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
                format,
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
                stride: std::mem::size_of::<heightmap::Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float4,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let (vertex_data, height_index_data) = heightmap::create_vertices(width_n, height_n);
        let height_vertex_buf = device
            .create_buffer_mapped(vertex_data.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&vertex_data);

        let height_index_buf = device
            .create_buffer_mapped(height_index_data.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&height_index_data);

        let height_index_count = height_index_data.len();

        HeightmapGpu {
            pipeline,
            bind_group,
            height_vertex_buf,
            height_index_buf,
            height_index_count,
        }
    }

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, main_bind_group, &[]);
        rpass.set_bind_group(1, &self.bind_group, &[]);

        rpass.set_index_buffer(&self.height_index_buf, 0);
        rpass.set_vertex_buffers(0, &[(&self.height_vertex_buf, 0)]);

        let nb_chunks = 1_u32; //self.game_state.debug_i1 as u32;
        let chunk_size = ((self.height_index_count as u32 / nb_chunks) / 3) * 3;
        for k in 0..nb_chunks {
            let range =
                (chunk_size * k)..(chunk_size) * (k + 1).min(self.height_index_count as u32);
            rpass.draw_indexed(range, 0, 0..1);
        }

        //        rpass.draw_indexed(0..(self.height_index_count) as u32, 0, 0..1);
    }
}

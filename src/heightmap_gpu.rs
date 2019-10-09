use crate::glsl_compiler;
use crate::heightmap;

use wgpu::{BindGroup, BindGroupLayout, RenderPass, RenderPipeline, Texture, TextureFormat};
use wgpu::{CommandEncoder, Device};

pub struct HeightmapGpu {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: usize,
    pub width: u32,
    pub height: u32,
    pub texels: Vec<f32>,
    ring_size: u32,
    texture: Texture,
    uniform_buf: wgpu::Buffer,
}

impl HeightmapGpu {
    pub fn new(
        device: &Device,
        init_encoder: &mut CommandEncoder,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
        width: u32,
        height: u32,
    ) -> Self {
        let texture_view_checker = {
            let size = 2u32;
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
            texture.create_default_view()
        };

        let sampler_checker = device.create_sampler(&wgpu::SamplerDescriptor {
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

        let start = std::time::Instant::now();
        let texels = heightmap::create_texels(width, height, 0.0);
        println!("texels took {}us", start.elapsed().as_micros());

        let texture_extent = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsage::SAMPLED,
        });

        let temp_buf = device
            .create_buffer_mapped(texels.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&texels);

        init_encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                row_pitch: 4 * width,
                image_height: height,
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

        let texture_view_height = texture.create_default_view();

        let sampler_height = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::Always,
        });

        //Map size
        let ring_size = 128;
        let map_size_cam_pos = [width as f32, height as f32, ring_size as f32, 0.0, 0.0];

        let uniform_buf = device
            .create_buffer_mapped(5, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&map_size_cam_pos);

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
                wgpu::BindGroupLayoutBinding {
                    binding: 3,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 4,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
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
                        range: 0..20,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view_checker),
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler_checker),
                },
                wgpu::Binding {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&texture_view_height),
                },
                wgpu::Binding {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler_height),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[main_bind_group_layout, &bind_group_layout],
        });

        // Create the render pipeline
        let vs_bytes = glsl_compiler::load(
            include_str!("shader/heightmap.vert"),
            glsl_compiler::ShaderStage::Vertex,
        );
        let fs_bytes = glsl_compiler::load(
            include_str!("shader/heightmap.frag"),
            glsl_compiler::ShaderStage::Fragment,
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
                    format: wgpu::VertexFormat::Float2,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let (vertex_data, height_index_data) = heightmap::create_vertex_index_rings(ring_size);
        //            heightmap::create_vertices_indices(width, height, 0.0);
        let vertex_buf = device
            .create_buffer_mapped(vertex_data.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&vertex_data);

        let index_buf = device
            .create_buffer_mapped(height_index_data.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&height_index_data);

        let index_count = height_index_data.len();

        HeightmapGpu {
            pipeline,
            bind_group,
            vertex_buf,
            index_buf,
            index_count,
            width,
            height,
            texels,
            ring_size,
            texture,
            uniform_buf,
        }
    }

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, main_bind_group, &[]);
        rpass.set_bind_group(1, &self.bind_group, &[]);
        rpass.set_index_buffer(&self.index_buf, 0);
        rpass.set_vertex_buffers(0, &[(&self.vertex_buf, 0)]);
        rpass.draw_indexed(0..(self.index_count) as u32, 0, 0..1);
    }

    pub fn update_uniform(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder,
        camera_x: f32,
        camera_y: f32,
    ) {
        let half_ring = self.ring_size as f32 / 2.0_f32;
        //Map size
        let map_size_cam_pos = [
            self.width as f32,
            self.height as f32,
            self.ring_size as f32,
            (camera_x.max(0.0).min(self.width as f32) / 1.0).floor() * 1.0,
            (camera_y.max(0.0).min(self.height as f32) / 1.0).floor() * 1.0,
        ];

        let uniform_buf = device
            .create_buffer_mapped(5, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&map_size_cam_pos);

        encoder.copy_buffer_to_buffer(&uniform_buf, 0, &self.uniform_buf, 0, 64);
    }

    pub fn get_z(&self, x: f32, y: f32) -> f32 {
        let i = x as usize + (y as usize) * self.width as usize;
        self.texels[i]
    }

    pub fn update(
        &mut self,
        min_x: u32,
        min_y: u32,
        width: u32,
        height: u32,
        editions: Vec<f32>,
        device: &Device,
        encoder: &mut CommandEncoder,
    ) {
        for i in min_x..min_x + width {
            for j in min_y..min_y + height {
                let ind_local = (i - min_x) + (j - min_y) * width;

                let ind_global = i + j * self.width;
                self.texels[ind_global as usize] = editions[ind_local as usize];
            }
        }

        let temp_buf = device
            .create_buffer_mapped(editions.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&editions);

        let texture_extent = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };

        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                row_pitch: 4 * width,
                image_height: height,
            },
            wgpu::TextureCopyView {
                texture: &self.texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d {
                    x: min_x as f32,
                    y: min_y as f32,
                    z: 0.0,
                },
            },
            texture_extent,
        );
    }
}

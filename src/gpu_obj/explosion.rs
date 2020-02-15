use super::glsl_compiler;
use crate::model;
use wgpu::Device;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, Texture, TextureFormat, TextureView};

pub struct ExplosionGpu {
    instance_buf: wgpu::Buffer,
    instance_count: u32,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    noise_texture: Texture,
}

impl ExplosionGpu {
    pub fn new(
        init_encoder: &mut wgpu::CommandEncoder,
        device: &Device,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
        current_position_att: &TextureView,
        normal_att: &TextureView,
    ) -> Self {
        log::trace!("ExplosionGpu new");

        let size = 256u32;
        let texels = Self::open_noise();
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

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler,
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 3,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler,
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 4,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 5,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler,
                },
            ],
        });

        let noise_texture_view = texture.create_default_view();
        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &noise_texture_view,
            current_position_att,
            normal_att,
        );

        let positions: Vec<f32> = Vec::new();

        let instance_buf = device
            .create_buffer_mapped(
                positions.len(),
                wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&positions);

        let pipeline =
            Self::create_pipeline(device, &bind_group_layout, main_bind_group_layout, format)
                .unwrap();

        ExplosionGpu {
            instance_buf,
            instance_count: 0,
            pipeline,
            bind_group,
            bind_group_layout,
            noise_texture: texture,
        }
    }

    pub fn create_bind_group(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        noise_texture_view: &TextureView,
        current_position_att: &TextureView,
        normal_att: &TextureView,
    ) -> BindGroup {
        // Create other resources
        let sampler_noise = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::Always,
        });

        let sampler_position = device.create_sampler(&wgpu::SamplerDescriptor {
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

        let sampler_normal = device.create_sampler(&wgpu::SamplerDescriptor {
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

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(noise_texture_view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler_noise),
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(current_position_att),
                },
                wgpu::Binding {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler_position),
                },
                wgpu::Binding {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(normal_att),
                },
                wgpu::Binding {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&sampler_normal),
                },
            ],
        })
    }

    pub fn update_bind_group(
        &mut self,
        device: &Device,
        current_position_att: &TextureView,
        normal_att: &TextureView,
    ) {
        let noise_texture_view = self.noise_texture.create_default_view();
        self.bind_group = Self::create_bind_group(
            device,
            &self.bind_group_layout,
            &noise_texture_view,
            current_position_att,
            normal_att,
        );
    }

    pub fn create_pipeline(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) -> glsl_compiler::Result<wgpu::RenderPipeline> {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&main_bind_group_layout, &bind_group_layout],
        });
        // Create the render pipeline
        let vs_bytes = glsl_compiler::load("./src/shader/explosion.vert")?;
        let fs_bytes = glsl_compiler::load("./src/shader/explosion.frag")?;

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
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: (4 * (6)) as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float,
                        offset: 4 * 3,
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

    pub fn open_noise() -> Vec<u8> {
        use byteorder::{BigEndian, ReadBytesExt};
        use std::fs::File;

        // The decoder is a build for reader and can be used to set various decoding options
        // via `Transformations`. The default output transformation is `Transformations::EXPAND
        // | Transformations::STRIP_ALPHA`.
        let mut decoder = png::Decoder::new(File::open(r"src/asset/2d/noise.png").unwrap());
        decoder.set_transformations(png::Transformations::IDENTITY);
        let (info, mut reader) = decoder.read_info().unwrap();

        // Display image metadata.
        log::debug!("info: {:?}", info.width);
        log::debug!("height: {:?}", info.height);
        log::debug!("bit depth: {:?}", info.bit_depth);
        log::debug!("buffer size: {:?}", info.buffer_size());

        // Allocate the output buffer.
        let mut buf = vec![0; info.buffer_size()];
        // Read the next frame. Currently this function should only called once.
        // The default options
        reader.next_frame(&mut buf).unwrap();
        buf
    }

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        log::trace!("ExplosionGpu render");
        if self.instance_count > 0 {
            rpass.set_pipeline(&self.pipeline);
            rpass.set_vertex_buffers(0, &[(&self.instance_buf, 0)]);
            rpass.set_bind_group(0, main_bind_group, &[]);
            rpass.set_bind_group(1, &self.bind_group, &[]);
            rpass.draw(0..4, 0..self.instance_count as u32);
        }
    }

    pub fn update_instance(&mut self, instance_attr: &[f32], device: &wgpu::Device) {
        log::trace!("ExplosionGpu update_instance");
        let temp_buf = device
            .create_buffer_mapped(instance_attr.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(instance_attr);

        std::mem::replace(&mut self.instance_buf, temp_buf);
        self.instance_count = instance_attr.len() as u32 / 6;
    }
}

impl super::trait_gpu::TraitGpu for ExplosionGpu {
    fn reload_shader(
        &mut self,
        device: &Device,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) {
        match Self::create_pipeline(
            device,
            &self.bind_group_layout,
            main_bind_group_layout,
            format,
        ) {
            Ok(pipeline) => self.pipeline = pipeline,
            Err(x) => log::error!("{}", x),
        };
    }
}

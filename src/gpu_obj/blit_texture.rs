use super::glsl_compiler;
use crate::model;
use crate::utils::ImageRGBA8;
use wgpu::Device;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, Texture, TextureFormat, TextureView};
pub struct BlitTextureGpu {
    instance_buf: wgpu::Buffer,
    instance_count: u32,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    noise_texture: Texture,
}

impl BlitTextureGpu {
    pub fn new(
        init_encoder: &mut wgpu::CommandEncoder,
        device: &Device,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
        img: ImageRGBA8,
    ) -> Self {
        log::trace!("BlitTextureGpu new");

        let texels = img.data;
        let texture_extent = wgpu::Extent3d {
            width: img.w,
            height: img.h,
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
                row_pitch: 4 * img.w,
                image_height: img.h,
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
            ],
        });

        let noise_texture_view = texture.create_default_view();
        let bind_group = Self::create_bind_group(device, &bind_group_layout, &noise_texture_view);

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
        BlitTextureGpu {
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
            ],
        })
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
        let vs_bytes = glsl_compiler::load("./src/shader/blit_texture.vert")?;
        let fs_bytes = glsl_compiler::load("./src/shader/blit_texture.frag")?;

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
                stride: (4 * (8)) as wgpu::BufferAddress,
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
                        format: wgpu::VertexFormat::Float2,
                        offset: 4 * 4,
                        shader_location: 2,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 4 * 6,
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
        log::trace!("BlitTextureGpu render");
        if self.instance_count > 0 {
            rpass.set_pipeline(&self.pipeline);
            rpass.set_vertex_buffers(0, &[(&self.instance_buf, 0)]);
            rpass.set_bind_group(0, main_bind_group, &[]);
            rpass.set_bind_group(1, &self.bind_group, &[]);
            rpass.draw(0..4, 0..self.instance_count as u32);
        }
    }

    pub fn update_instance(&mut self, instance_attr: &[f32], device: &wgpu::Device) {
        log::trace!("BlitTextureGpu update_instance");
        let temp_buf = device
            .create_buffer_mapped(instance_attr.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(instance_attr);

        std::mem::replace(&mut self.instance_buf, temp_buf);
        self.instance_count = instance_attr.len() as u32 / 8;
    }
}

impl super::trait_gpu::TraitGpu for BlitTextureGpu {
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

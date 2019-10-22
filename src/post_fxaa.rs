use crate::glsl_compiler;
use wgpu::Device;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, TextureFormat, TextureView};

pub struct PostFxaa {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl PostFxaa {
    pub fn new(
        device: &Device,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) -> Self {
        // Create pipeline layout
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
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

        let pipeline =
            Self::create_pipeline(device, &bind_group_layout, main_bind_group_layout, format)
                .unwrap();
        PostFxaa {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    fn create_pipeline(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) -> glsl_compiler::Result<wgpu::RenderPipeline> {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&main_bind_group_layout, &bind_group_layout],
        });

        // Create the render pipeline
        let vs_bytes = glsl_compiler::load("shader/post.vert", glsl_compiler::ShaderStage::Vertex)?;
        let fs_bytes = glsl_compiler::load(
            "shader/post_fxaa.frag",
            glsl_compiler::ShaderStage::Fragment,
        )?;
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
                format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
        Ok(pipeline)
    }

    pub fn render(
        &self,
        rpass: &mut RenderPass,
        device: &Device,
        main_bind_group: &BindGroup,
        last_pass_view: &TextureView,
    ) {
        log::trace!("PostFxaa render");
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &main_bind_group, &[]);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&last_pass_view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        rpass.set_bind_group(1, &bind_group, &[]);
        rpass.draw(0..4, 0..1);
    }
}

impl crate::trait_gpu::TraitGpu for PostFxaa {
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
            Err(x) => println!("{}", x),
        };
    }
}

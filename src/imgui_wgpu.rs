use imgui::{Context, DrawCmd::Elements, DrawIdx, DrawList, DrawVert, TextureId, Textures, Ui};
use std::mem::size_of;
use wgpu::*;

pub type RendererResult<T> = Result<T, RendererError>;

#[derive(Clone, Debug)]
pub enum RendererError {
    BadTexture(TextureId),
}

fn get_program_link() -> (&'static str, &'static str) {
    (("./src/shader/imgui.vert"), ("./src/shader/imgui.frag"))
}

/// A container for a bindable texture to be used internally.
struct Texture {
    bind_group: BindGroup,
}

impl Texture {
    /// Creates a new imgui texture from a wgpu texture.
    fn new(texture: wgpu::Texture, layout: &BindGroupLayout, device: &Device) -> Self {
        // Extract the texture view.
        let view = texture.create_default_view();

        // Create the texture sampler.
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: CompareFunction::Always,
        });

        // Create the texture bind group from the layout.
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout,
            bindings: &[
                Binding {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                Binding {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Texture { bind_group }
    }
}

#[allow(dead_code)]
pub struct Renderer {
    pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
    textures: Textures<Texture>,
    texture_layout: BindGroupLayout,
    clear_color: Option<Color>,
}

impl Renderer {
    /// Create an entirely new imgui wgpu renderer.
    pub fn new(
        imgui: &mut Context,
        device: &mut Device,
        format: TextureFormat,
        clear_color: Option<Color>,
    ) -> Renderer {
        let (vs_code, fs_code) = get_program_link();
        let vs_raw = crate::glsl_compiler::load(vs_code).unwrap();
        let fs_raw = crate::glsl_compiler::load(fs_code).unwrap();
        Self::new_impl(imgui, device, format, clear_color, vs_raw, fs_raw)
    }

    /// Create an entirely new imgui wgpu renderer.
    fn new_impl(
        imgui: &mut Context,
        device: &mut Device,
        format: TextureFormat,
        clear_color: Option<Color>,
        vs_raw: Vec<u32>,
        fs_raw: Vec<u32>,
    ) -> Renderer {
        // Load shaders.
        let vs_module = device.create_shader_module(&vs_raw);
        let fs_module = device.create_shader_module(&fs_raw);

        // Create the uniform matrix buffer.
        let size = 64;
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            size,
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        // Create the uniform matrix buffer bind group layout.
        let uniform_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            bindings: &[BindGroupLayoutBinding {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: BindingType::UniformBuffer { dynamic: false },
            }],
        });

        // Create the uniform matrix buffer bind group.
        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &uniform_layout,
            bindings: &[Binding {
                binding: 0,
                resource: BindingResource::Buffer {
                    buffer: &uniform_buffer,
                    range: 0..size,
                },
            }],
        });

        // Create the texture layout for further usage.
        let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            bindings: &[
                BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: BindingType::SampledTexture {
                        multisampled: false,
                        dimension: TextureViewDimension::D2,
                    },
                },
                BindGroupLayoutBinding {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler,
                },
            ],
        });

        // Create the render pipeline layout.
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[&uniform_layout, &texture_layout],
        });

        // Create the render pipeline.
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(RasterizationStateDescriptor {
                front_face: FrontFace::Cw,
                cull_mode: CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: PrimitiveTopology::TriangleList,
            color_states: &[ColorStateDescriptor {
                format,
                color_blend: BlendDescriptor {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha_blend: BlendDescriptor {
                    src_factor: BlendFactor::OneMinusDstAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                write_mask: ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: IndexFormat::Uint16,
            vertex_buffers: &[VertexBufferDescriptor {
                stride: size_of::<DrawVert>() as BufferAddress,
                step_mode: InputStepMode::Vertex,
                attributes: &[
                    VertexAttributeDescriptor {
                        format: VertexFormat::Float2,
                        shader_location: 0,
                        offset: 0,
                    },
                    VertexAttributeDescriptor {
                        format: VertexFormat::Float2,
                        shader_location: 1,
                        offset: 8,
                    },
                    VertexAttributeDescriptor {
                        format: VertexFormat::Uint,
                        shader_location: 2,
                        offset: 16,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let mut renderer = Renderer {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            textures: Textures::new(),
            texture_layout,
            clear_color,
        };

        // Immediately load the fon texture to the GPU.
        renderer.reload_font_texture(imgui, device);

        renderer
    }

    /// Render the current imgui frame.
    pub fn render<'a>(
        &mut self,
        ui: Ui<'a>,
        device: &Device,
        encoder: &mut CommandEncoder,
        view: &TextureView,
    ) -> RendererResult<()> {
        let draw_data = ui.render();
        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        // If the render area is <= 0, exit here and now.
        if !(fb_width > 0.0 && fb_height > 0.0) {
            return Ok(());
        }

        let width = draw_data.display_size[0];
        let height = draw_data.display_size[1];

        // Create and update the transform matrix for the current frame.
        // This is required to adapt to vulkan coordinates.
        let matrix = [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 2.0 / height as f32, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [-1.0, -1.0, 0.0, 1.0],
        ];
        self.update_uniform_buffer(device, encoder, &matrix);

        // Start a new renderpass and prepare it properly.
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachmentDescriptor {
                attachment: &view,
                resolve_target: None,
                load_op: match self.clear_color {
                    Some(_) => LoadOp::Clear,
                    _ => LoadOp::Load,
                },
                store_op: StoreOp::Store,
                clear_color: self.clear_color.unwrap_or(Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }),
            }],
            depth_stencil_attachment: None,
        });
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        // Execute all the imgui render work.
        for draw_list in draw_data.draw_lists() {
            self.render_draw_list(
                device,
                &mut rpass,
                &draw_list,
                draw_data.display_pos,
                draw_data.framebuffer_scale,
            )?;
        }

        Ok(())
    }

    /// Render a given `DrawList` from imgui onto a wgpu frame.
    fn render_draw_list<'render>(
        &mut self,
        device: &Device,
        rpass: &mut RenderPass<'render>,
        draw_list: &DrawList,
        clip_off: [f32; 2],
        clip_scale: [f32; 2],
    ) -> RendererResult<()> {
        let mut start = 0;

        // Make sure the current buffers are uploaded to the GPU.
        let vertex_buffer = self.upload_vertex_buffer(device, draw_list.vtx_buffer());
        let index_buffer = self.upload_index_buffer(device, draw_list.idx_buffer());

        // Make sure the current buffers are attached to the render pass.
        rpass.set_index_buffer(&index_buffer, 0);
        rpass.set_vertex_buffers(0, &[(&vertex_buffer, 0)]);
        for cmd in draw_list.commands() {
            match cmd {
                Elements { count, cmd_params } => {
                    let clip_rect = [
                        (cmd_params.clip_rect[0] - clip_off[0]) * clip_scale[0],
                        (cmd_params.clip_rect[1] - clip_off[1]) * clip_scale[1],
                        (cmd_params.clip_rect[2] - clip_off[0]) * clip_scale[0],
                        (cmd_params.clip_rect[3] - clip_off[1]) * clip_scale[1],
                    ];

                    // Set the current texture bind group on the renderpass.
                    let texture_id = cmd_params.texture_id.into();
                    let tex = self
                        .textures
                        .get(texture_id)
                        .ok_or_else(|| RendererError::BadTexture(texture_id))?;
                    rpass.set_bind_group(1, &tex.bind_group, &[]);

                    // Set scissors on the renderpass.
                    let scissors = (
                        clip_rect[0].max(0.0).floor() as u32,
                        clip_rect[1].max(0.0).floor() as u32,
                        (clip_rect[2] - clip_rect[0]).abs().ceil() as u32,
                        (clip_rect[3] - clip_rect[1]).abs().ceil() as u32,
                    );
                    rpass.set_scissor_rect(scissors.0, scissors.1, scissors.2, scissors.3);

                    // Draw the current batch of vertices with the renderpass.
                    let end = start + count as u32;
                    rpass.draw_indexed(start..end, 0, 0..1);
                    start = end;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Updates the current uniform buffer containing the transform matrix.
    fn update_uniform_buffer(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder,
        matrix: &[[f32; 4]; 4],
    ) {
        // Create a new buffer.
        let buffer = device
            .create_buffer_mapped(16, BufferUsage::COPY_SRC)
            .fill_from_slice(
                matrix
                    .iter()
                    .flatten()
                    .map(|f| *f)
                    .collect::<Vec<f32>>()
                    .as_slice(),
            );
        // Copy the new buffer to the real buffer.
        encoder.copy_buffer_to_buffer(&buffer, 0, &self.uniform_buffer, 0, 64);
    }

    /// Upload the vertex buffer to the gPU.
    fn upload_vertex_buffer(&mut self, device: &Device, vertices: &[DrawVert]) -> Buffer {
        device
            .create_buffer_mapped(vertices.len(), BufferUsage::VERTEX)
            .fill_from_slice(vertices)
    }

    /// Upload the index buffer to the GPU.
    fn upload_index_buffer(&mut self, device: &Device, indices: &[DrawIdx]) -> Buffer {
        device
            .create_buffer_mapped(indices.len(), BufferUsage::INDEX)
            .fill_from_slice(indices)
    }

    /// Updates the texture on the GPU corresponding to the current imgui font atlas.
    ///
    /// This has to be called after loading a font.
    pub fn reload_font_texture(&mut self, imgui: &mut Context, device: &mut Device) {
        let mut atlas = imgui.fonts();
        let handle = atlas.build_rgba32_texture();
        let font_texture_id =
            self.upload_font_texture(device, &handle.data, handle.width, handle.height);
        atlas.tex_id = font_texture_id;
    }

    /// Creates and uploads a new wgpu texture made from the imgui font atlas.
    fn upload_font_texture(
        &mut self,
        device: &mut Device,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> TextureId {
        // Create the wgpu texture.
        let texture = device.create_texture(&TextureDescriptor {
            size: Extent3d {
                width,
                height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        });

        // Upload the actual data to a wgpu buffer.
        let bytes = data.len();
        let buffer = device
            .create_buffer_mapped(bytes, BufferUsage::COPY_SRC)
            .fill_from_slice(data);

        // Make sure we have an active encoder.
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { todo: 0 });

        // Schedule a copy from the buffer to the texture.
        encoder.copy_buffer_to_texture(
            BufferCopyView {
                buffer: &buffer,
                offset: 0,
                row_pitch: bytes as u32 / height,
                image_height: height,
            },
            TextureCopyView {
                texture: &texture,
                mip_level: 0,
                array_layer: 0,
                origin: Origin3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
            Extent3d {
                width,
                height,
                depth: 1,
            },
        );

        // Resolve the actual copy process.
        device.get_queue().submit(&[encoder.finish()]);

        let texture = Texture::new(texture, &self.texture_layout, device);
        self.textures.insert(texture)
    }
}

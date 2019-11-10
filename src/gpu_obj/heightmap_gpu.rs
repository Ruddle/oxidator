use super::glsl_compiler;
use super::heightmap_helper;
use crate::heightmap_phy;

use wgpu::{BindGroup, BindGroupLayout, RenderPass, RenderPipeline, Texture, TextureFormat};
use wgpu::{CommandEncoder, Device};

const ZONE_SIZE_MIP0: usize = 64;
const UPDATE_PER_STEP: usize = 300;
const MIP_COUNT: u32 = 5;
pub const MAX_Z: f32 = 511.0;

pub struct HeightmapGpu {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: usize,
    pub phy: heightmap_phy::HeightmapPhy,
    ring_size: u32,
    texture: Texture,
    texture_lod: Texture,
    uniform_buf: wgpu::Buffer,
    zone_to_update_mip0: Vec<i32>,
    zone_to_update_mip1: Vec<i32>,
    zone_to_update_mip2: Vec<i32>,
    mip4_to_update: bool,
}

impl HeightmapGpu {
    pub fn new(
        device: &Device,
        init_encoder: &mut CommandEncoder,
        format: TextureFormat,
        main_bind_group_layout: &BindGroupLayout,
        phy: heightmap_phy::HeightmapPhy,
    ) -> Self {
        log::trace!("HeightmapGpu new");
        let texture_view_checker = {
            let size = 2u32;
            let texels = crate::procedural_texels::checker(size as usize);
            let texture_extent = wgpu::Extent3d {
                width: size,
                height: size,
                depth: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                size: texture_extent,
                array_layer_count: 1,
                mip_level_count: 2,
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

            {
                let texture_extent = wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth: 1,
                };
                let temp_buf = device
                    .create_buffer_mapped(4, wgpu::BufferUsage::COPY_SRC)
                    .fill_from_slice(&[123, 123, 123, 255]);
                init_encoder.copy_buffer_to_texture(
                    wgpu::BufferCopyView {
                        buffer: &temp_buf,
                        offset: 0,
                        row_pitch: 4 * 1,
                        image_height: 1,
                    },
                    wgpu::TextureCopyView {
                        texture: &texture,
                        mip_level: 1,
                        array_layer: 0,
                        origin: wgpu::Origin3d {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                    },
                    texture_extent,
                );
            }

            texture.create_default_view()
        };

        let sampler_checker = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::Always,
        });

        let (texture_view_lod, texture_lod) = {
            let width = phy.width as u32 / ZONE_SIZE_MIP0 as u32;
            let height = phy.height as u32 / ZONE_SIZE_MIP0 as u32;

            let size = phy.width as u32 * phy.height as u32;
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
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            });

            let mut texels = vec![0_f32; size as usize];

            texels[0] = 4.0;

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

            (texture.create_default_view(), texture)
        };

        let sampler_lod = device.create_sampler(&wgpu::SamplerDescriptor {
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

        let texture_extent = wgpu::Extent3d {
            width: phy.width as u32,
            height: phy.height as u32,
            depth: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            array_layer_count: 1,
            mip_level_count: MIP_COUNT,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        let temp_buf = device
            .create_buffer_mapped(phy.texels.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&phy.texels);

        init_encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                row_pitch: 4 * phy.width as u32,
                image_height: phy.height as u32,
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

        let mut mipmaper = |mip: u32| {
            let m = 2_u32.pow(mip);

            let width = phy.width as u32 / m;
            let height = phy.height as u32 / m;
            let texture_extent = wgpu::Extent3d {
                width,
                height,
                depth: 1,
            };
            let mut texels2 = Vec::new();
            for j in 0..height {
                for i in 0..width {
                    texels2.push(phy.texels[(i * m + (j * m) * width * m) as usize]);
                }
            }

            let temp_buf = device
                .create_buffer_mapped(texels2.len(), wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&texels2);

            init_encoder.copy_buffer_to_texture(
                wgpu::BufferCopyView {
                    buffer: &temp_buf,
                    offset: 0,
                    row_pitch: 4 * width,
                    image_height: height,
                },
                wgpu::TextureCopyView {
                    texture: &texture,
                    mip_level: mip,
                    array_layer: 0,
                    origin: wgpu::Origin3d {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                },
                texture_extent,
            );
        };

        for i in 1..MIP_COUNT {
            mipmaper(i);
        }

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
        let map_size_cam_pos = [
            phy.width as f32,
            phy.height as f32,
            ring_size as f32,
            0.0,
            0.0,
        ];

        let uniform_buf = device
            .create_buffer_mapped(5, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&map_size_cam_pos);

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
                wgpu::BindGroupLayoutBinding {
                    binding: 5,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 6,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler,
                },
            ],
        });

        let pipeline =
            Self::create_pipeline(device, &bind_group_layout, main_bind_group_layout, format)
                .unwrap();

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
                wgpu::Binding {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&texture_view_lod),
                },
                wgpu::Binding {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&sampler_lod),
                },
            ],
        });

        let (vertex_data, height_index_data) =
            heightmap_helper::create_vertex_index_rings(ring_size);
        //            heightmap::create_vertices_indices(width, height, 0.0);
        let vertex_buf = device
            .create_buffer_mapped(vertex_data.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&vertex_data);

        let index_buf = device
            .create_buffer_mapped(height_index_data.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&height_index_data);

        let index_count = height_index_data.len();

        let mut zone_to_update_mip0 = Vec::new();

        for _ in (0..=phy.width).step_by(ZONE_SIZE_MIP0) {
            for _ in (0..=phy.height).step_by(ZONE_SIZE_MIP0) {
                zone_to_update_mip0.push(0);
            }
        }

        let mut zone_to_update_mip1 = Vec::new();

        for _ in (0..=phy.width).step_by(ZONE_SIZE_MIP0 * 2) {
            for _ in (0..=phy.height).step_by(ZONE_SIZE_MIP0 * 2) {
                zone_to_update_mip1.push(0);
            }
        }

        let mut zone_to_update_mip2 = Vec::new();
        for _ in (0..=phy.width).step_by(ZONE_SIZE_MIP0 * 4) {
            for _ in (0..=phy.height).step_by(ZONE_SIZE_MIP0 * 4) {
                zone_to_update_mip2.push(0);
            }
        }

        HeightmapGpu {
            pipeline,
            bind_group,
            bind_group_layout,
            vertex_buf,
            index_buf,
            index_count,
            phy,
            ring_size,
            texture,
            texture_lod,
            uniform_buf,
            zone_to_update_mip0,
            zone_to_update_mip1,
            zone_to_update_mip2,
            mip4_to_update: false,
        }
    }

    pub fn create_pipeline(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        main_bind_group_layout: &BindGroupLayout,
        format: TextureFormat,
    ) -> glsl_compiler::Result<wgpu::RenderPipeline> {
        // Create pipeline layout

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[main_bind_group_layout, bind_group_layout],
        });

        // Create the render pipeline
        let vs_bytes = glsl_compiler::load("./src/shader/heightmap.vert")?;
        let fs_bytes = glsl_compiler::load("./src/shader/heightmap.frag")?;
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
                    format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
                wgpu::ColorStateDescriptor {
                    format: wgpu::TextureFormat::Rgba32Float,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
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
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<heightmap_helper::Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float,
                        offset: 4 * 2,
                        shader_location: 1,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
        Ok(pipeline)
    }

    pub fn mipmap_update(
        &self,
        mip: u32,
        device: &Device,
        encoder: &mut CommandEncoder,
        texture: &Texture,
        texels: &[f32],
        texels_width: u32,
        i: u32,
        j: u32,
        width: u32,
        height: u32,
    ) {
        let m = 2_u32.pow(mip);

        let width = width / m;
        let height = height / m;

        let texture_extent = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };

        let min_x = i / m;
        let min_y = j / m;

        let mut texels2 = Vec::new();
        for j in min_y..(min_y + height) {
            for i in min_x..(min_x + width) {
                texels2.push(texels[(i * m + (j * m) * (texels_width / m) * m) as usize]);
            }
        }

        let temp_buf = device
            .create_buffer_mapped(texels2.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&texels2);

        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                row_pitch: 4 * width,
                image_height: height,
            },
            wgpu::TextureCopyView {
                texture,
                mip_level: mip,
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

    pub fn render(&self, rpass: &mut RenderPass, main_bind_group: &BindGroup) {
        log::trace!("HeightmapGpu render");
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
        log::trace!("HeightmapGpu update_uniform");
        //Map size
        let map_size_cam_pos = [
            self.phy.width as u32 as f32,
            self.phy.height as u32 as f32,
            self.ring_size as f32,
            (camera_x.max(0.0).min(self.phy.width as u32 as f32) / 1.0),
            (camera_y.max(0.0).min(self.phy.height as u32 as f32) / 1.0),
        ];

        let uniform_buf = device
            .create_buffer_mapped(
                5,
                wgpu::BufferUsage::UNIFORM
                    | wgpu::BufferUsage::COPY_DST
                    | wgpu::BufferUsage::COPY_SRC,
            )
            .fill_from_slice(&map_size_cam_pos);

        encoder.copy_buffer_to_buffer(&uniform_buf, 0, &self.uniform_buf, 0, 64);
    }

    pub fn step(&mut self, device: &Device, encoder: &mut CommandEncoder) {
        let mut update_left = UPDATE_PER_STEP;
        if self.mip4_to_update {
            self.mip4_to_update = false;
            update_left = update_left / 2;
            for mip in 3..MIP_COUNT {
                self.mipmap_update(
                    mip,
                    device,
                    encoder,
                    &self.texture,
                    &self.phy.texels,
                    self.phy.width as u32,
                    0,
                    0,
                    self.phy.width as u32,
                    self.phy.height as u32,
                );
            }
        }

        if update_left > 0 {
            let mut zone_to_update_mip2 = self
                .zone_to_update_mip2
                .iter()
                .enumerate()
                .filter(|(_, b)| **b != 0)
                .collect::<Vec<(usize, &i32)>>();

            let update_to_do = zone_to_update_mip2
                .len()
                .min(UPDATE_PER_STEP)
                .min(update_left);
            update_left -= update_to_do;
            zone_to_update_mip2.sort_by_key(|(_, i)| **i);

            let indices: Vec<usize> = zone_to_update_mip2
                .iter()
                .map(|(index, _)| *index)
                .collect();

            for index in indices.iter().skip(update_to_do) {
                self.zone_to_update_mip2[*index] -= 1;
            }

            for index in indices.iter().take(update_to_do) {
                self.zone_to_update_mip2[*index] = 0;

                let i = *index as u32 % (self.phy.width as u32 / (ZONE_SIZE_MIP0 * 4) as u32);
                let j = *index as u32 / (self.phy.width as u32 / (ZONE_SIZE_MIP0 * 4) as u32);
                let min_x = i * ZONE_SIZE_MIP0 as u32 * 4;
                let min_y = j * ZONE_SIZE_MIP0 as u32 * 4;

                if min_x < self.phy.width as u32 && min_y < self.phy.height as u32 {
                    let width = (ZONE_SIZE_MIP0 as u32 * 4).min(self.phy.width as u32 - min_x);
                    let height = (ZONE_SIZE_MIP0 as u32 * 4).min(self.phy.height as u32 - min_y);

                    self.mipmap_update(
                        2,
                        device,
                        encoder,
                        &self.texture,
                        &self.phy.texels,
                        self.phy.width as u32,
                        min_x,
                        min_y,
                        width,
                        height,
                    );
                }
            }
        }

        if update_left > 0 {
            let mut zone_to_update_mip1 = self
                .zone_to_update_mip1
                .iter()
                .enumerate()
                .filter(|(_, b)| **b != 0)
                .collect::<Vec<(usize, &i32)>>();

            let update_to_do = zone_to_update_mip1
                .len()
                .min(UPDATE_PER_STEP)
                .min(update_left);
            update_left -= update_to_do;
            zone_to_update_mip1.sort_by_key(|(_, i)| **i);

            let indices: Vec<usize> = zone_to_update_mip1
                .iter()
                .map(|(index, _)| *index)
                .collect();

            for index in indices.iter().skip(update_to_do) {
                self.zone_to_update_mip1[*index] -= 1;
            }

            for index in indices.iter().take(update_to_do) {
                self.zone_to_update_mip1[*index] = 0;

                let i = *index as u32 % (self.phy.width as u32 / (ZONE_SIZE_MIP0 * 2) as u32);
                let j = *index as u32 / (self.phy.width as u32 / (ZONE_SIZE_MIP0 * 2) as u32);
                let min_x = i * ZONE_SIZE_MIP0 as u32 * 2;
                let min_y = j * ZONE_SIZE_MIP0 as u32 * 2;

                if min_x < self.phy.width as u32 && min_y < self.phy.height as u32 {
                    let width = (ZONE_SIZE_MIP0 as u32 * 2).min(self.phy.width as u32 - min_x);
                    let height = (ZONE_SIZE_MIP0 as u32 * 2).min(self.phy.height as u32 - min_y);

                    self.mipmap_update(
                        1,
                        device,
                        encoder,
                        &self.texture,
                        &self.phy.texels,
                        self.phy.width as u32,
                        min_x,
                        min_y,
                        width,
                        height,
                    );
                }
            }
        }

        if update_left > 0 {
            let mut zone_to_update_mip0 = self
                .zone_to_update_mip0
                .iter()
                .enumerate()
                .filter(|(_, b)| **b != 0)
                .collect::<Vec<(usize, &i32)>>();

            let update_to_do = zone_to_update_mip0
                .len()
                .min(UPDATE_PER_STEP)
                .min(update_left);
            update_left -= update_to_do;

            zone_to_update_mip0.sort_by_key(|(_, i)| **i);

            let indices: Vec<usize> = zone_to_update_mip0
                .iter()
                .map(|(index, _)| *index)
                .collect();

            for index in indices.iter().skip(update_to_do) {
                self.zone_to_update_mip0[*index] -= 1;
            }

            for index in indices.iter().take(update_to_do) {
                self.zone_to_update_mip0[*index] = 0;

                let i = *index as u32 % (self.phy.width as u32 / ZONE_SIZE_MIP0 as u32);
                let j = *index as u32 / (self.phy.width as u32 / ZONE_SIZE_MIP0 as u32);
                let min_x = i * ZONE_SIZE_MIP0 as u32;
                let min_y = j * ZONE_SIZE_MIP0 as u32;

                if min_x < self.phy.width as u32 && min_y < self.phy.height as u32 {
                    let width = (ZONE_SIZE_MIP0 as u32).min(self.phy.width as u32 - min_x);
                    let height = (ZONE_SIZE_MIP0 as u32).min(self.phy.height as u32 - min_y);

                    self.mipmap_update(
                        0,
                        device,
                        encoder,
                        &self.texture,
                        &self.phy.texels,
                        self.phy.width as u32,
                        min_x,
                        min_y,
                        width,
                        height,
                    );
                }
            }
        }
        //Update lod texture

        let width = self.phy.width as u32 / ZONE_SIZE_MIP0 as u32;
        let height = self.phy.height as u32 / ZONE_SIZE_MIP0 as u32;

        let size = width * height;
        let texture_extent = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };

        let mut lod = vec![3.0f32; size as usize];
        for j in 0..height as usize {
            for i in 0..width as usize {
                if self.zone_to_update_mip0[i + j * width as usize] == 0 {
                    lod[i + j * width as usize] = 0.0;
                } else if self.zone_to_update_mip1[i / 2 + (j / 2) * width as usize / 2] == 0 {
                    lod[i + j * width as usize] = 1.0;
                } else if self.zone_to_update_mip2[i / 4 + (j / 4) * (width as usize / 4)] == 0 {
                    lod[i + j * width as usize] = 2.0;
                }
            }
        }

        let temp_buf = device
            .create_buffer_mapped(lod.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&lod);

        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                row_pitch: 4 * width,
                image_height: height,
            },
            wgpu::TextureCopyView {
                texture: &self.texture_lod,
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
    }

    pub fn update_rect(&mut self, min_x: u32, min_y: u32, width: u32, height: u32) {
        for i in (min_x / ZONE_SIZE_MIP0 as u32)..=(min_x + width) / ZONE_SIZE_MIP0 as u32 {
            for j in (min_y / ZONE_SIZE_MIP0 as u32)..=(min_y + height) / ZONE_SIZE_MIP0 as u32 {
                let width = self.phy.width as u32 / ZONE_SIZE_MIP0 as u32;

                let rank = &mut self.zone_to_update_mip2[(i / 4 + (j / 4) * (width / 4)) as usize];
                if *rank == 0 {
                    *rank = -1;
                }

                let rank = &mut self.zone_to_update_mip1[(i / 2 + (j / 2) * (width / 2)) as usize];
                if *rank == 0 {
                    *rank = -1;
                }

                let rank = &mut self.zone_to_update_mip0[(i + j * width) as usize];
                if *rank == 0 {
                    *rank = -1;
                }
            }
        }

        self.mip4_to_update = true;
    }
}

impl super::trait_gpu::TraitGpu for HeightmapGpu {
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

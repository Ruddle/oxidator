mod camera;
mod fake_texels;
mod framework;
mod game_state;
mod glsl_compiler;
mod heightmap;
mod heightmap_gpu;
mod model;
mod model_gpu;
extern crate nalgebra as na;
use na::{Matrix4, Point3, Rotation3, Vector3};

use heightmap_gpu::HeightmapGpu;
use imgui::*;
use imgui_wgpu::Renderer;
use imgui_winit_support;
use imgui_winit_support::WinitPlatform;
use model_gpu::ModelGpu;
use std::time::Instant;
use wgpu::TextureFormat;

struct App {
    forward_depth: wgpu::TextureView,
    heightmap_gpu: HeightmapGpu,
    cube_gpu: ModelGpu,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    format: TextureFormat,
    uniform_buf: wgpu::Buffer,
    screen_res: (u32, u32),
    game_state: game_state::State,
    imgui_wrap: ImguiWrap,
}

struct ImguiWrap {
    imgui: Context,
    platform: WinitPlatform,
    renderer: Renderer,
}

impl App {}

impl framework::App for App {
    fn init(
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &mut wgpu::Device,
        window: &winit::window::Window,
        hidpi_factor: f64,
    ) -> (Self, Option<wgpu::CommandBuffer>) {
        use std::mem;

        let mut init_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
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

        // Create the texture
        let size = 256u32;
        let texels = fake_texels::create_texels(size as usize);
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
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            address_mode_w: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::Always,
        });
        let mx_total = camera::create_view_proj(
            sc_desc.width as f32 / sc_desc.height as f32,
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        let mx_ref: &[f32] = mx_total.as_slice();
        let uniform_buf = device
            .create_buffer_mapped(16, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(mx_ref);

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &uniform_buf,
                        range: 0..64,
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

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: sc_desc.width,
                height: sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        let imgui_wrap = {
            // imgui
            let mut imgui = imgui::Context::create();
            let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
            platform.attach_window(
                imgui.io_mut(),
                window,
                imgui_winit_support::HiDpiMode::Default,
            );
            imgui.set_ini_filename(None);

            let font_size = (13.0 * hidpi_factor) as f32;
            imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

            imgui.fonts().add_font(&[FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

            // imgui <-> wgpu
            let renderer = Renderer::new(&mut imgui, device, sc_desc.format, None);

            ImguiWrap {
                imgui,
                platform,
                renderer,
            }
        };

        let format: TextureFormat = sc_desc.format;

        let heightmap_gpu =
            HeightmapGpu::new(device, &mut init_encoder, format, &bind_group_layout, 1, 1);

        let cube_gpu = ModelGpu::new(
            &model::create_cube(),
            device,
            &mut init_encoder,
            format,
            &bind_group_layout,
        );

        // Done
        let this = App {
            bind_group_layout,
            bind_group,
            uniform_buf,
            format,
            cube_gpu,
            heightmap_gpu,
            forward_depth: depth_texture.create_default_view(),
            screen_res: (sc_desc.width, sc_desc.height),
            game_state: game_state::State::new(),
            imgui_wrap,
        };
        (this, Some(init_encoder.finish()))
    }

    fn resize(
        &mut self,
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
    ) -> Option<wgpu::CommandBuffer> {
        self.screen_res = (sc_desc.width, sc_desc.height);
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: sc_desc.width,
                height: sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });
        self.forward_depth = depth_texture.create_default_view();
        None
    }

    fn update(&mut self, _event: &winit::event::Event<()>, window: &winit::window::Window) {
        use winit::event;
        use winit::event::WindowEvent;
        self.imgui_wrap
            .platform
            .handle_event(self.imgui_wrap.imgui.io_mut(), window, _event);

        match _event {
            event::Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                virtual_keycode: Some(vkc),
                                state: event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        self.game_state.key_pressed.insert(vkc.clone());
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                virtual_keycode: Some(vkc),
                                state: event::ElementState::Released,
                                ..
                            },
                        ..
                    } => {
                        self.game_state.key_pressed.remove(vkc);
                    }

                    WindowEvent::MouseWheel {
                        delta: event::MouseScrollDelta::LineDelta(dx, dy),
                        ..
                    } => {
                        self.game_state.last_scroll = *dy;
                    }
                    _ => {}
                };
            }
            _ => {}
        }
    }

    fn render(
        &mut self,
        frame: &wgpu::SwapChainOutput,
        device: &mut wgpu::Device,
        window: &winit::window::Window,
    ) -> wgpu::CommandBuffer {
        let mut now = Instant::now();
        let mut delta = now - self.game_state.last_frame;
        let last_compute_time = delta.clone();

        //empiric, a feed back loop could find this value automatically
        let oversleep = 60;
        let min_us = 1000000_u64 / self.game_state.fps;
        let min_wait_until_next_frame = std::time::Duration::from_micros(min_us - oversleep);
        if min_wait_until_next_frame > delta {
            std::thread::sleep(min_wait_until_next_frame - delta);
        }

        now = Instant::now();
        delta = now - self.game_state.last_frame;
        self.game_state.last_frame = now;
        let last_compute_time_total = delta.clone();

        let delta_sim_sec = last_compute_time_total.as_secs_f32();

        // Movements
        {
            use winit::event::VirtualKeyCode as Key;
            let key_pressed = &self.game_state.key_pressed;
            let on = |vkc| key_pressed.contains(&vkc);

            let mut offset = Vector3::new(0.0, 0.0, 0.0);
            let mut rotation = self.game_state.dir.clone();

            let k = (if !on(Key::LShift) { 1.0 } else { 2.0 }) * self.game_state.position.z;
            //Game
            if on(Key::S) {
                offset.y -= k;
            }
            if on(Key::Z) {
                offset.y += k;
            }
            if on(Key::Q) {
                offset.x -= k;
            }
            if on(Key::D) {
                offset.x += k;
            }

            if on(Key::LControl) {
                if self.game_state.last_scroll > 0.0 {
                    rotation.y += 1.0
                }
                if self.game_state.last_scroll < 0.0 {
                    rotation.z -= 1.0
                }
            } else {
                offset.z = -self.game_state.last_scroll * k * 20.0;
            }

            self.game_state.last_scroll = 0.0;

            self.game_state.position += offset * delta_sim_sec;
            self.game_state.dir =
                (self.game_state.dir + rotation * 33.0 * delta_sim_sec).normalize();

            self.game_state.position_smooth += (self.game_state.position.coords
                - self.game_state.position_smooth.coords)
                * delta_sim_sec.min(0.033)
                * 15.0;

            self.game_state.dir_smooth += (self.game_state.dir - self.game_state.dir_smooth)
                * delta_sim_sec.min(0.033)
                * 15.0;
        }

        //Render
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        camera::update_camera_uniform(
            self.screen_res,
            &self.game_state.position_smooth,
            &self.game_state.dir_smooth,
            &self.uniform_buf,
            device,
            &mut encoder,
        );

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.forward_depth,
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
            });

            self.heightmap_gpu.render(&mut rpass, &self.bind_group);
            self.cube_gpu.render(&mut rpass, &self.bind_group);
        }

        //Imgui

        {
            self.imgui_wrap
                .platform
                .prepare_frame(self.imgui_wrap.imgui.io_mut(), &window)
                .expect("Failed to prepare frame");

            let ui = self.imgui_wrap.imgui.frame();

            {
                let mut_fps = &mut self.game_state.fps;
                let debug_i1 = &mut self.game_state.debug_i1;
                let mut rebuild_heightmap = false;
                let window = imgui::Window::new(im_str!("Statistics"));
                window
                    .size([400.0, 200.0], Condition::FirstUseEver)
                    .position([3.0, 3.0], Condition::FirstUseEver)
                    .build(&ui, || {
                        imgui::Slider::new(im_str!("fps"), 1..=240).build(&ui, mut_fps);
                        ui.text(im_str!("Frametime: {}us", last_compute_time.as_micros()));
                        ui.text(im_str!(
                            " \" Capped: {}us",
                            last_compute_time_total.as_micros()
                        ));

                        if imgui::Slider::new(im_str!("debug_i1"), (1..=1000)).build(&ui, debug_i1)
                        {
                            rebuild_heightmap = true;
                        }
                    });

                if true || rebuild_heightmap {
                    //                    let heightmap_gpu = HeightmapGpu::new(
                    //                        device,
                    //                        &mut encoder,
                    //                        self.format,
                    //                        &self.bind_group_layout,
                    //                        *debug_i1 as u32,
                    //                        32,
                    //                    );
                    let t = self.game_state.start_time.elapsed().as_secs_f32();

                    let mut positions = Vec::with_capacity((*debug_i1 * *debug_i1 * 3) as usize);
                    for i in 0..*debug_i1 {
                        for j in 0..*debug_i1 {
                            positions.push(0.5 + (2 * i) as f32);
                            positions.push(0.5 + (2 * j) as f32);
                            positions.push(
                                10.0 + 3.0
                                    * f32::sin(
                                        (1.0 + 2.0 * i as f32 / (*debug_i1 as f32))
                                            * (1.0 + 2.0 * j as f32 / (*debug_i1 as f32))
                                            * t,
                                    ),
                            );
                        }
                    }

                    self.cube_gpu
                        .update_instance(&positions[..], &mut encoder, device);

                    //                    std::mem::replace(&mut self.heightmap_gpu, heightmap_gpu);
                }
            }
            self.imgui_wrap.platform.prepare_render(&ui, window);
            self.imgui_wrap
                .renderer
                .render(ui, device, &mut encoder, &frame.view)
                .expect("Rendering failed");
        }

        encoder.finish()
    }
}

fn main() {
    framework::run::<App>("Oxidator");
}

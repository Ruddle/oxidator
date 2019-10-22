use crate::*;

use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};

use gpu;
use heightmap_gpu::HeightmapGpu;
use imgui::*;
use imgui_wgpu::Renderer;
use imgui_winit_support;
use imgui_winit_support::WinitPlatform;
use model_gpu::ModelGpu;
mod misc;
mod play;
mod render;

use crate::heightmap_phy;
use log::info;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use utils::time;
use wgpu::{BufferMapAsyncResult, Extent3d, SwapChain, TextureFormat};

use winit::event::WindowEvent;

pub enum FromClient {
    Event(frame::FrameEvent),
}

struct ImguiWrap {
    imgui: imgui::Context,
    platform: WinitPlatform,
    renderer: Renderer,
}

#[derive(Clone)]
enum RenderEvent {
    ChangeMode { from: MainMode, to: MainMode },
}

#[derive(PartialEq, Clone, Copy)]
pub enum MainMode {
    Home,
    Play,
    MapEditor,
}

pub struct App {
    //Wgpu
    gpu: gpu::WgpuState,
    //Physics
    phy_state: phy_state::State,

    first_color_att_view: wgpu::TextureView,
    forward_depth: wgpu::TextureView,
    position_att: wgpu::Texture,
    position_att_view: wgpu::TextureView,

    heightmap_gpu: HeightmapGpu,
    cube_gpu: ModelGpu,
    kbot_gpu: ModelGpu,
    kinematic_projectile_gpu: ModelGpu,

    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,

    ub_camera_mat: wgpu::Buffer,
    ub_misc: wgpu::Buffer,

    postfx: post_fx::PostFx,
    postfxaa: post_fxaa::PostFxaa,

    game_state: game_state::State,
    input_state: input_state::InputState,
    imgui_wrap: ImguiWrap,

    main_menu: MainMode,

    sender_to_client: crossbeam_channel::Sender<ToClient>,
    receiver_to_client: crossbeam_channel::Receiver<ToClient>,

    sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,

    sender_from_client: crossbeam_channel::Sender<FromClient>,

    receiver_notify: crossbeam_channel::Receiver<notify::Result<notify::event::Event>>,
    watcher: notify::RecommendedWatcher,

    mailbox: Vec<RenderEvent>,
}

impl App {
    pub fn new(
        window: winit::window::Window,
        sender_to_client: crossbeam_channel::Sender<ToClient>,
        receiver_to_client: crossbeam_channel::Receiver<ToClient>,
        sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,
        sender_from_client: crossbeam_channel::Sender<FromClient>,
    ) -> (Self) {
        log::trace!("App init");

        let mut gpu = gpu::WgpuState::new(window);

        let mut init_encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        wgpu::BindGroupLayoutBinding {
                            binding: 3,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
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
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        let texture_view = texture.create_default_view();
        let temp_buf = gpu
            .device
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
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
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
            gpu.sc_desc.width as f32 / gpu.sc_desc.height as f32,
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        let mx_ref: &[f32] = mx_total.as_slice();
        let ub_camera_mat = gpu
            .device
            .create_buffer_mapped(16, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(mx_ref);

        //2 Mouse pos
        //2 resolution
        let ub_misc = gpu
            .device
            .create_buffer_mapped(8, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&[0.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

        // Create bind group
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &ub_camera_mat,
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
                wgpu::Binding {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &ub_misc,
                        range: 0..(8 * 4),
                    },
                },
            ],
        });

        log::trace!("   imgui_wrap init");
        let imgui_wrap = {
            // imgui
            let mut imgui = imgui::Context::create();
            let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
            platform.attach_window(
                imgui.io_mut(),
                &gpu.window,
                imgui_winit_support::HiDpiMode::Default,
            );
            imgui.set_ini_filename(None);

            let font_size = (13.0 * gpu.hidpi_factor) as f32;
            imgui.io_mut().font_global_scale = (1.0 / gpu.hidpi_factor) as f32;

            imgui.fonts().add_font(&[FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

            // imgui <-> wgpu
            let renderer = Renderer::new(&mut imgui, &mut gpu.device, gpu.sc_desc.format, None);

            ImguiWrap {
                imgui,
                platform,
                renderer,
            }
        };

        let format: TextureFormat = gpu.sc_desc.format;

        let heightmap_gpu = HeightmapGpu::new(
            &gpu.device,
            &mut init_encoder,
            format,
            &bind_group_layout,
            heightmap_phy::HeightmapPhy::new(2048, 2048),
        );

        let cube_gpu = ModelGpu::new(
            &model::create_cube(),
            &gpu.device,
            format,
            &bind_group_layout,
        );

        let kbot_gpu = ModelGpu::new(
            &model::create_cube(),
            &gpu.device,
            format,
            &bind_group_layout,
        );

        let kinematic_projectile_gpu = ModelGpu::new(
            &model::open_arrow(),
            &gpu.device,
            format,
            &bind_group_layout,
        );

        let depth_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: gpu.sc_desc.width,
                height: gpu.sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        let position_att = gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: gpu.sc_desc.width,
                height: gpu.sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });

        let postfx = post_fx::PostFx::new(&gpu.device, &bind_group_layout, format);
        let postfxaa = post_fxaa::PostFxaa::new(&gpu.device, &bind_group_layout, format);

        gpu.device.get_queue().submit(&[init_encoder.finish()]);

        let mut game_state = game_state::State::new();

        println!("Number of mobiles {}", game_state.kbots.len());

        let (receiver_notify, watcher) = {
            use crossbeam_channel::unbounded;
            use notify::{watcher, RecursiveMode, Result};
            use std::time::Duration;
            let (tx, rx) = unbounded();
            use notify::Watcher;

            // Automatically select the best implementation for your platform.
            // You can also access each implementation directly e.g. INotifyWatcher.
            let mut watcher = watcher(tx, Duration::from_millis(500)).unwrap();

            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            watcher
                .watch(std::env::current_dir().unwrap(), RecursiveMode::Recursive)
                .unwrap();
            (rx, watcher)
        };

        let first_color_att = gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: gpu.sc_desc.width,
                height: gpu.sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });

        let first_color_att_view = first_color_att.create_default_view();

        // Done
        let this = App {
            gpu,
            phy_state: phy_state::State::new(),

            bind_group,
            bind_group_layout,
            ub_camera_mat,
            ub_misc,
            cube_gpu,
            kbot_gpu,
            kinematic_projectile_gpu,
            heightmap_gpu,
            first_color_att_view,
            forward_depth: depth_texture.create_default_view(),
            position_att_view: position_att.create_default_view(),
            position_att,

            postfx,
            postfxaa,

            game_state,
            input_state: input_state::InputState::new(),
            imgui_wrap,
            main_menu: MainMode::Home,

            sender_to_client,
            receiver_to_client,
            sender_to_event_loop,
            sender_from_client,
            receiver_notify,
            watcher,

            mailbox: Vec::new(),
        };

        (this)
    }

    fn resize(&mut self) -> Option<wgpu::CommandBuffer> {
        log::trace!("resize");

        let first_color_att = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: self.gpu.sc_desc.width,
                height: self.gpu.sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });

        self.first_color_att_view = first_color_att.create_default_view();

        let depth_texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: self.gpu.sc_desc.width,
                height: self.gpu.sc_desc.height,
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

        let position_att = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: self.gpu.sc_desc.width,
                height: self.gpu.sc_desc.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });

        self.position_att_view = position_att.create_default_view();
        self.position_att = position_att;

        None
    }

    pub fn handle_winit_event(&mut self, _event: &winit::event::Event<()>) {
        log::trace!("[client.rs] update {:?}", _event);
        use winit::event;

        self.imgui_wrap.platform.handle_event(
            self.imgui_wrap.imgui.io_mut(),
            &self.gpu.window,
            _event,
        );

        //Low level
        match _event {
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let physical = size.to_physical(self.gpu.hidpi_factor);
                info!("Resizing to {:?}", physical);
                self.gpu.sc_desc.width = physical.width.round() as u32;
                self.gpu.sc_desc.height = physical.height.round() as u32;
                self.gpu.swap_chain = self
                    .gpu
                    .device
                    .create_swap_chain(&self.gpu.surface, &self.gpu.sc_desc);
                let command_buf = self.resize();
                if let Some(command_buf) = command_buf {
                    self.gpu.device.get_queue().submit(&[command_buf]);
                }
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    self.sender_to_event_loop.send(EventLoopMsg::Stop).unwrap();
                }
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(vkc),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    self.input_state.key_pressed.insert(vkc.clone());
                    self.input_state.key_trigger.insert(vkc.clone());
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
                    self.input_state.key_pressed.remove(vkc);
                    self.input_state.key_release.insert(vkc.clone());
                }

                WindowEvent::MouseWheel {
                    delta: event::MouseScrollDelta::LineDelta(_, dy),
                    ..
                } => {
                    self.input_state.last_scroll = *dy;
                }

                WindowEvent::CursorMoved { position, .. } => {
                    self.input_state.cursor_pos = (position.x as u32, position.y as u32);
                    match self.input_state.drag {
                        input_state::Drag::Start { x0, y0 }
                        | input_state::Drag::Dragging { x0, y0, .. } => {
                            self.input_state.drag = input_state::Drag::Dragging {
                                x0,
                                y0,
                                x1: self.input_state.cursor_pos.0 as u32,
                                y1: self.input_state.cursor_pos.1 as u32,
                            };
                        }
                        _ => {}
                    }
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    if !self.imgui_wrap.imgui.io().want_capture_mouse {
                        if let &winit::event::ElementState::Pressed = state {
                            self.input_state.mouse_pressed.insert(*button);
                            self.input_state.mouse_trigger.insert(*button);

                            if let event::MouseButton::Left = button {
                                self.input_state.drag = input_state::Drag::Start {
                                    x0: self.input_state.cursor_pos.0 as u32,
                                    y0: self.input_state.cursor_pos.1 as u32,
                                }
                            };
                        } else {
                            self.input_state.mouse_pressed.remove(button);
                            self.input_state.mouse_release.insert(*button);
                            if let event::MouseButton::Left = button {
                                match self.input_state.drag {
                                    input_state::Drag::Dragging { x0, y0, .. } => {
                                        self.input_state.drag = input_state::Drag::End {
                                            x0,
                                            y0,
                                            x1: self.input_state.cursor_pos.0 as u32,
                                            y1: self.input_state.cursor_pos.1 as u32,
                                        };
                                    }
                                    _ => {
                                        self.input_state.drag = input_state::Drag::None;
                                    }
                                }
                            }
                        }
                    }
                }

                _ => {}
            },
            event::Event::EventsCleared => {
                //                self.render();
            }
            _ => (),
        }
    }

    pub fn map_read_async_msg(&mut self, vec: Vec<f32>, usage: String) {
        let to_update = match usage.as_ref() {
            "screen_center_world_pos" => &mut self.game_state.screen_center_world_pos,
            "mouse_world_pos" => &mut self.game_state.mouse_world_pos,
            _ => &mut self.game_state.mouse_world_pos,
        };

        if vec.len() == 4 && vec[0] >= 0.0 {
            std::mem::replace(to_update, Some(Vector3::new(vec[0], vec[1], vec[2])));
        } else {
            std::mem::replace(to_update, None);
        }
    }

    pub fn receive(&mut self) {
        let msg: std::result::Result<
            notify::Result<notify::event::Event>,
            crossbeam_channel::TryRecvError,
        > = self.receiver_notify.try_recv();
        match msg {
            Ok(Ok(event)) => {
                println!("notify {:?}", event);

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "post_ui.frag" || name.to_os_string() == "post.vert"
                    })
                }) {
                    println!("Reloading post.vert/post_ui.frag");
                    self.postfx.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "post_fxaa.frag"
                            || name.to_os_string() == "post.vert"
                    })
                }) {
                    println!("Reloading post.vert/post_fxaa.frag");
                    self.postfxaa.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "heightmap.frag"
                            || name.to_os_string() == "heightmap.vert"
                    })
                }) {
                    println!("Reloading heightmap.vert/heightmap.frag");
                    self.heightmap_gpu.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }
            }
            _ => {}
        }

        match self.receiver_to_client.try_recv() {
            Ok(x) => {
                log::trace!("receive: {:?}", x);

                match x {
                    ToClient::EventMessage { event } => {
                        self.handle_winit_event(&event);
                    }
                    ToClient::MapReadAsyncMessage { vec, usage } => {
                        self.map_read_async_msg(vec, usage);
                    }
                    ToClient::Render => {
                        self.render();
                    }
                    ToClient::NewFrame(frame) => {
                        self.game_state.handle_new_frame(frame);
                    }
                }
            }
            _ => {
                log::trace!("No message yo");
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    }
}

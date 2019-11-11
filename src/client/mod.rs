use crate::*;

use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};

use gpu_obj::imgui_wgpu::Renderer;

use gpu_obj::arrow_gpu::ArrowGpu;
use gpu_obj::gpu;
use gpu_obj::heightmap_gpu::HeightmapGpu;
use gpu_obj::model_gpu::ModelGpu;
use gpu_obj::trait_gpu::TraitGpu;
use gpu_obj::water::WaterGpu;
use imgui::*;
use imgui_winit_support;
use imgui_winit_support::WinitPlatform;
mod camera;
mod game_state;

mod unit_editor;

mod heightmap_editor;
mod input_state;
mod misc;
mod play;
mod render;

use crate::heightmap_phy;
use log::info;
use spin_sleep::LoopHelper;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use utils::time;
use wgpu::{BufferMapAsyncResult, Extent3d, SwapChain, TextureFormat};
use winit::event::WindowEvent;

pub struct StartClient {
    pub bind: String,
}

pub struct StartServer {
    pub bind: String,
}

pub enum FromClient {
    PlayerInput(frame::FrameEventFromPlayer),
    StartServer(StartServer),
    StartClient(StartClient),
    DisconnectServer,
    DisconnectClient,
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
    UnitEditor,
    MapEditor,
    MultiplayerLobby,
}

#[derive(PartialEq, Clone, Copy)]
pub enum NetMode {
    Offline,
    Server,
    Client,
}

pub enum GenericGpuState {
    ToLoad(model::TriangleList),
    Ready(ModelGpu),
    Error(String),
}

pub struct App {
    //Wgpu
    gpu: gpu::WgpuState,

    first_color_att_view: wgpu::TextureView,
    secon_color_att_view: wgpu::TextureView,
    forward_depth: wgpu::TextureView,
    position_att: wgpu::Texture,
    position_att_view: wgpu::TextureView,

    heightmap_gpu: HeightmapGpu,
    water_gpu: WaterGpu,
    generic_gpu: HashMap<PathBuf, GenericGpuState>,

    kbot_gpu: ModelGpu,
    arrow_gpu: ArrowGpu,
    kinematic_projectile_gpu: ModelGpu,
    vertex_attr_buffer_f32: Vec<f32>,

    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,

    ub_camera_mat: wgpu::Buffer,
    ub_misc: wgpu::Buffer,

    postfx: gpu_obj::post_fx::PostFx,
    postfxaa: gpu_obj::post_fxaa::PostFxaa,
    post_bicopy: gpu_obj::texture_view_bicopy::TextureViewBiCopy,
    health_bar: gpu_obj::health_bar::HealthBarGpu,
    line_gpu: gpu_obj::line::LineGpu,
    unit_icon: gpu_obj::unit_icon::UnitIconGpu,
    explosion_gpu: gpu_obj::explosion::ExplosionGpu,

    game_state: game_state::State,
    input_state: input_state::InputState,
    imgui_wrap: ImguiWrap,

    main_menu: MainMode,
    net_mode: NetMode,

    unit_editor: unit_editor::UnitEditor,

    sender_to_client: crossbeam_channel::Sender<ToClient>,
    receiver_to_client: crossbeam_channel::Receiver<ToClient>,

    sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,

    sender_from_client_to_manager: crossbeam_channel::Sender<FromClient>,

    receiver_notify: crossbeam_channel::Receiver<notify::Result<notify::event::Event>>,
    watcher: notify::RecommendedWatcher,

    mailbox: Vec<RenderEvent>,

    loop_helper: LoopHelper,
    profiler: frame::ProfilerMap,
    global_info: Option<manager::GlobalInfo>,
    threadpool: rayon::ThreadPool,
}

impl App {
    pub fn new(
        window: winit::window::Window,
        sender_to_client: crossbeam_channel::Sender<ToClient>,
        receiver_to_client: crossbeam_channel::Receiver<ToClient>,

        sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,
        sender_from_client_to_manager: crossbeam_channel::Sender<FromClient>,
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
                            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                        },
                    ],
                });

        // Create the texture
        let size = 256u32;
        let texels = procedural_texels::create_texels(size as usize);
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
            1.0,
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        let mx_ref: &[f32] = mx_total.as_slice();

        let mx_view =
            camera::create_view(&Point3::new(0.0, 0.0, 0.0), &Vector3::new(0.0, 0.0, 0.0));
        let mx_view_ref: &[f32] = mx_view.as_slice();

        let mx_normal =
            camera::create_normal(&Point3::new(0.0, 0.0, 0.0), &Vector3::new(0.0, 0.0, 0.0));
        let mx_normal_ref: &[f32] = mx_normal.as_slice();

        let mut filler = Vec::new();
        filler.extend_from_slice(mx_ref);
        filler.extend_from_slice(mx_view_ref);
        //TODO reuse camera.rs code
        filler.extend_from_slice(mx_ref);
        filler.extend_from_slice(mx_normal_ref);
        let ub_camera_mat = gpu
            .device
            .create_buffer_mapped(
                16 * 4,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&filler[..]);

        //2 Mouse pos
        //2 resolution
        let ub_misc = gpu
            .device
            .create_buffer_mapped(10, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&[0.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

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
                        range: 0..(10 * 4),
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
                imgui_winit_support::HiDpiMode::Rounded,
            );
            imgui.set_ini_filename(None);

            let font_size = (13.0 * gpu.hidpi_factor) as f32;
            imgui.io_mut().font_global_scale = (1.0) as f32;

            imgui.io_mut().mouse_draw_cursor = true;

            imgui.fonts().add_font(&[FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

            // imgui <-> wgpu
            let renderer = Renderer::new(
                &mut imgui,
                &mut gpu.device,
                &mut gpu.queue,
                gpu.sc_desc.format,
                None,
            );

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

        let kbot_gpu = ModelGpu::new(
            // &crate::model::open_obj("./src/asset/tank/tank-base.obj"), //
            &model::open_obj("./src/asset/cube.obj").unwrap(),
            &gpu.device,
            format,
            &bind_group_layout,
        );

        let kinematic_projectile_gpu = ModelGpu::new(
            &model::open_obj("./src/asset/small_sphere.obj").unwrap(),
            &gpu.device,
            format,
            &bind_group_layout,
        );

        let arrow_gpu = ArrowGpu::new(
            &model::open_obj("./src/asset/arrow.obj").unwrap(),
            &gpu.device,
            format,
            &bind_group_layout,
        );

        let mut generic_gpu = HashMap::new();

        generic_gpu.insert(
            Path::new("./src/asset/cube.obj").to_owned(),
            GenericGpuState::ToLoad(model::open_obj("./src/asset/cube.obj").unwrap()),
        );

        generic_gpu.insert(
            Path::new("./src/asset/small_sphere.obj").to_owned(),
            GenericGpuState::ToLoad(model::open_obj("./src/asset/small_sphere.obj").unwrap()),
        );

        let health_bar =
            gpu_obj::health_bar::HealthBarGpu::new(&gpu.device, format, &bind_group_layout);

        let line_gpu = gpu_obj::line::LineGpu::new(&gpu.device, format, &bind_group_layout);

        let unit_icon =
            gpu_obj::unit_icon::UnitIconGpu::new(&gpu.device, format, &bind_group_layout);

        let explosion_gpu = gpu_obj::explosion::ExplosionGpu::new(
            &mut init_encoder,
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
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
                | wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_SRC,
        });

        let position_att_view = position_att.create_default_view();

        gpu.queue.submit(&[init_encoder.finish()]);

        let game_state = game_state::State::new();

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

        let secon_color_att = gpu.device.create_texture(&wgpu::TextureDescriptor {
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

        let secon_color_att_view = secon_color_att.create_default_view();

        let water_gpu = WaterGpu::new(
            &gpu.device,
            format,
            &bind_group_layout,
            &secon_color_att_view,
            &position_att_view,
        );

        let postfx = gpu_obj::post_fx::PostFx::new(
            &gpu.device,
            &bind_group_layout,
            format,
            &position_att_view,
        );
        let postfxaa = gpu_obj::post_fxaa::PostFxaa::new(
            &gpu.device,
            &bind_group_layout,
            format,
            &first_color_att_view,
        );

        let post_bicopy = gpu_obj::texture_view_bicopy::TextureViewBiCopy::new(
            &gpu.device,
            &bind_group_layout,
            format,
            &secon_color_att_view,
        );

        // Done
        let this = App {
            gpu,

            bind_group,
            bind_group_layout,
            ub_camera_mat,
            ub_misc,
            kbot_gpu,
            generic_gpu,
            kinematic_projectile_gpu,
            arrow_gpu,
            heightmap_gpu,
            water_gpu,
            vertex_attr_buffer_f32: Vec::new(),

            first_color_att_view,
            secon_color_att_view,
            forward_depth: depth_texture.create_default_view(),
            position_att_view,
            position_att,

            postfx,
            postfxaa,
            post_bicopy,
            health_bar,
            line_gpu,
            unit_icon,
            explosion_gpu,

            game_state,
            input_state: input_state::InputState::new(),
            imgui_wrap,
            main_menu: MainMode::Home,
            net_mode: NetMode::Offline,
            unit_editor: unit_editor::UnitEditor::new(),

            sender_to_client,
            receiver_to_client,
            sender_to_event_loop,
            sender_from_client_to_manager,
            receiver_notify,
            watcher,

            mailbox: Vec::new(),

            loop_helper: LoopHelper::builder().build_with_target_rate(144.0),
            profiler: frame::ProfilerMap::new(),
            global_info: None,
            threadpool: rayon::ThreadPoolBuilder::new()
                // .num_threads(8)
                .build()
                .unwrap(),
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

        let secon_color_att = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
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

        self.secon_color_att_view = secon_color_att.create_default_view();

        self.postfxaa
            .update_last_pass_view(&self.gpu.device, &self.first_color_att_view);

        self.post_bicopy
            .update_last_pass_view(&self.gpu.device, &self.secon_color_att_view);

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
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
                | wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_SRC,
        });

        self.position_att_view = position_att.create_default_view();

        self.water_gpu.update_bind_group(
            &self.gpu.device,
            &self.secon_color_att_view,
            &self.position_att_view,
        );

        self.postfx
            .update_pos_att_view(&self.gpu.device, &self.position_att_view);
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
                    self.gpu.queue.submit(&[command_buf]);
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
                    let position = position.to_physical(self.gpu.hidpi_factor);

                    let (old_x, old_y) = self.input_state.cursor_pos;

                    self.input_state.cursor_offset = (
                        position.x as i32 - old_x as i32,
                        position.y as i32 - old_y as i32,
                    );
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

                _ => {
                    // log::warn!("{:?}", x);
                }
            },
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
                log::trace!("notify {:?}", event);

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "post_ui.frag" || name.to_os_string() == "post.vert"
                    })
                }) {
                    log::info!("Reloading post.vert/post_ui.frag");
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
                    log::info!("Reloading post.vert/post_fxaa.frag");
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
                    log::info!("Reloading heightmap.vert/heightmap.frag");
                    self.heightmap_gpu.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "cube_instanced.frag"
                            || name.to_os_string() == "cube_instanced.vert"
                    })
                }) {
                    log::info!("Reloading cube_instanced.vert/cube_instanced.frag");
                    self.kbot_gpu.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "arrow.frag" || name.to_os_string() == "arrow.vert"
                    })
                }) {
                    log::info!("Reloading arrow.vert/arrow.frag");
                    self.arrow_gpu.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "health_bar.frag"
                            || name.to_os_string() == "health_bar.vert"
                    })
                }) {
                    log::info!("Reloading health_bar.vert/health_bar.frag");
                    self.health_bar.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "unit_icon.frag"
                            || name.to_os_string() == "unit_icon.vert"
                    })
                }) {
                    log::info!("Reloading unit_icon.vert/unit_icon.frag");
                    self.unit_icon.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "explosion.frag"
                            || name.to_os_string() == "explosion.vert"
                    })
                }) {
                    log::info!("Reloading explosion.vert/explosion.frag");
                    self.explosion_gpu.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "line.frag" || name.to_os_string() == "line.vert"
                    })
                }) {
                    log::info!("Reloading line.vert/line.frag");
                    self.line_gpu.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }

                if event.paths.iter().any(|p| {
                    p.file_name().iter().any(|name| {
                        name.to_os_string() == "water.frag" || name.to_os_string() == "water.vert"
                    })
                }) {
                    log::info!("Reloading water.vert/water.frag");
                    self.water_gpu.reload_shader(
                        &self.gpu.device,
                        &self.bind_group_layout,
                        self.gpu.sc_desc.format,
                    );
                }
            }
            _ => {}
        }

        let msgs: Vec<_> = self.receiver_to_client.try_iter().collect();
        for msg in msgs {
            {
                match msg {
                    ToClient::MapReadAsyncMessage { vec, usage } => {
                        log::trace!("receive: MapReadAsyncMessage");
                        self.map_read_async_msg(vec, usage);
                    }
                    ToClient::NewFrame(frame) => {
                        self.game_state.handle_new_frame(frame);
                    }
                    ToClient::GlobalInfo(global_info) => self.global_info = Some(global_info),
                }
            }
        }
    }
}

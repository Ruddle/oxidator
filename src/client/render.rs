use super::client::*;
use crate::frame;
use crate::frame::FrameEventFromPlayer;
use crate::frame::Player;
use crate::*;
use imgui::*;
use na::{IsometryMatrix3, Matrix4, Point3, Vector2, Vector3, Vector4};
use std::time::Duration;
use std::time::Instant;
use utils::time;
use wgpu::{BufferMapAsyncResult, Extent3d};

impl App {
    pub fn render(&mut self) {
        let frame_time = self.game_state.last_frame.elapsed();
        self.profiler.mix("frame_time", frame_time, 20);

        log::trace!("sleep");
        self.loop_helper.loop_sleep();
        self.loop_helper.loop_start();
        log::trace!("render");

        let capped_frame_time = self.game_state.last_frame.elapsed();

        self.profiler
            .mix("capped_frame_time", capped_frame_time, 20);
        self.game_state.last_frame = Instant::now();

        let sim_sec = capped_frame_time.as_secs_f32();

        let mailbox = self.mailbox.clone();
        self.mailbox.clear();

        for mail in mailbox {
            match mail {
                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::MapEditor,
                } => {
                    self.clear_gpu_instance_and_game_state();
                    self.game_state.position = Point3::new(1024.0, 400.0, 1100.0);
                    self.game_state.dir = Vector3::new(0.0, 0.3, -1.0);
                }

                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::Play,
                } => {
                    self.init_play();
                }

                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::Home,
                } => {
                    self.game_state.position = Point3::new(200.0, 100.0, 50.0);
                    self.game_state.dir = Vector3::new(0.0, 0.3, -1.0);
                    match self.net_mode {
                        NetMode::Offline | NetMode::Server => {
                            let replacer = FrameEventFromPlayer::ReplaceFrame(frame::Frame::new());
                            let _ = self
                                .sender_from_client_to_manager
                                .try_send(client::FromClient::PlayerInput(replacer));
                        }
                        NetMode::Client => {}
                    }
                }
                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::MultiplayerLobby,
                } => {}
                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::UnitEditor,
                } => {
                    self.init_unit_editor();
                }
                _ => {
                    log::info!("Something else");
                }
            }
        }

        if self
            .input_state
            .key_trigger
            .contains(&winit::event::VirtualKeyCode::Escape)
        {
            let next_mode = MainMode::Home;
            if self.main_menu != next_mode {
                self.mailbox.push(RenderEvent::ChangeMode {
                    from: self.main_menu,
                    to: next_mode,
                });
                self.main_menu = next_mode;
            }
        }

        let mode_with_camera = [MainMode::Play, MainMode::MapEditor];
        // Camera Movements
        if mode_with_camera.contains(&self.main_menu) {
            self.rts_camera(sim_sec);
        }

        if self.main_menu == MainMode::UnitEditor {
            self.orbit_camera(sim_sec);
        }

        let heightmap_editor_duration = time(|| {
            if let MainMode::MapEditor = self.main_menu {
                if let Some(mouse_world_pos) = self.game_state.mouse_world_pos {
                    self.game_state.heightmap_editor.handle_user_input(
                        &self.input_state.mouse_pressed,
                        &mouse_world_pos,
                        &mut self.heightmap_gpu,
                    );
                }
            }
        });

        self.profiler
            .mix("heightmap_editor", heightmap_editor_duration, 20);

        //Render
        let mut encoder_render = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        //Load pending generic gpu
        for (path, generic_gpu_state) in self.generic_gpu.iter_mut() {
            if let GenericGpuState::ToLoad(tri_list) = generic_gpu_state {
                let mut generic_gpu = ModelGpu::new(
                    &tri_list,
                    &self.gpu.device,
                    self.gpu.sc_desc.format,
                    &self.bind_group_layout,
                );
                log::debug!("Load pending generic gpu {:?} ", path);
                let mut generic_gpu_state_new = GenericGpuState::Ready(generic_gpu);
                std::mem::replace(generic_gpu_state, generic_gpu_state_new);
            }
        }

        let view_proj = camera::create_view_proj(
            self.gpu.sc_desc.width as f32 / self.gpu.sc_desc.height as f32,
            &self.game_state.position_smooth,
            &self.game_state.dir_smooth,
        );
        if self.main_menu == MainMode::Play {
            self.handle_play(sim_sec, &mut encoder_render, &view_proj);
        }

        self.upload_to_gpu(&view_proj, &mut encoder_render);

        let heightmap_gpu_step_duration = time(|| {
            self.heightmap_gpu
                .step(&self.gpu.device, &mut encoder_render);
        });

        self.profiler
            .mix("heightmap_gpu_step", heightmap_gpu_step_duration, 20);

        let cursor_sample_position = self
            .gpu
            .device
            .create_buffer_mapped::<f32>(
                4,
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
            )
            .finish();

        let screen_center_sample_position = self
            .gpu
            .device
            .create_buffer_mapped::<f32>(
                4,
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
            )
            .finish();

        encoder_render.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &self.position_att,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d {
                    x: self
                        .input_state
                        .cursor_pos
                        .0
                        .max(0)
                        .min(self.gpu.sc_desc.width - 1) as f32,
                    y: self
                        .input_state
                        .cursor_pos
                        .1
                        .max(0)
                        .min(self.gpu.sc_desc.height - 1) as f32,
                    z: 0.0,
                },
            },
            wgpu::BufferCopyView {
                buffer: &cursor_sample_position,
                offset: 0,
                row_pitch: 4 * 4,
                image_height: 1,
            },
            Extent3d {
                width: 1,
                height: 1,
                depth: 1,
            },
        );

        encoder_render.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &self.position_att,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d {
                    x: self.gpu.sc_desc.width as f32 / 2.0,
                    y: self.gpu.sc_desc.height as f32 / 2.0,
                    z: 0.0,
                },
            },
            wgpu::BufferCopyView {
                buffer: &screen_center_sample_position,
                offset: 0,
                row_pitch: 4 * 4,
                image_height: 1,
            },
            Extent3d {
                width: 1,
                height: 1,
                depth: 1,
            },
        );

        let mut start_drag = (
            self.input_state.cursor_pos.0 as f32,
            self.input_state.cursor_pos.1 as f32,
        );

        if let MainMode::Play = self.main_menu {
            if let input_state::Drag::Dragging { x0, y0, .. } = self.input_state.drag {
                start_drag = (x0 as f32, y0 as f32);
            }
        }

        let radius = if self.main_menu == MainMode::MapEditor {
            self.game_state.heightmap_editor.pen_radius as f32
        } else {
            0.0
        };
        let ub_misc = self
            .gpu
            .device
            .create_buffer_mapped(
                10,
                wgpu::BufferUsage::UNIFORM
                    | wgpu::BufferUsage::COPY_DST
                    | wgpu::BufferUsage::COPY_SRC,
            )
            .fill_from_slice(&[
                self.input_state.cursor_pos.0 as f32,
                self.input_state.cursor_pos.1 as f32,
                self.gpu.sc_desc.width as f32,
                self.gpu.sc_desc.height as f32,
                1.0 / self.gpu.sc_desc.width as f32,
                1.0 / self.gpu.sc_desc.height as f32,
                start_drag.0,
                start_drag.1,
                radius,
                self.game_state.heightmap_editor.pen_strength as f32,
            ]);

        encoder_render.copy_buffer_to_buffer(&ub_misc, 0, &self.ub_misc, 0, 10 * 4);

        self.heightmap_gpu.update_uniform(
            &self.gpu.device,
            &mut encoder_render,
            self.game_state.position_smooth.x,
            self.game_state.position_smooth.y,
        );

        camera::update_camera_uniform(
            (self.gpu.sc_desc.width, self.gpu.sc_desc.height),
            &self.game_state.position_smooth,
            &self.game_state.dir_smooth,
            &self.ub_camera_mat,
            &self.gpu.device,
            &mut encoder_render,
        );

        //Imgui
        let start = Instant::now();

        log::trace!("imgui render");
        self.imgui_wrap
            .platform
            .prepare_frame(self.imgui_wrap.imgui.io_mut(), &self.gpu.window)
            .expect("Failed to prepare frame");

        let ui: Ui = self.imgui_wrap.imgui.frame();
        {
            let main_menu = &mut self.main_menu;

            {
                //Stat
                let fps_before = self.game_state.fps.clone();
                let mut_fps = &mut self.game_state.fps;
                let profiler_logic = &self.game_state.frame_zero.frame_profiler;
                let profiler_render = &self.profiler;
                let stats_window = imgui::Window::new(im_str!("Statistics"));
                stats_window
                    .size([300.0, 400.0], imgui::Condition::FirstUseEver)
                    .position([3.0, 3.0], imgui::Condition::FirstUseEver)
                    .collapsed(true, imgui::Condition::FirstUseEver)
                    .resizable(true)
                    .movable(false)
                    .build(&ui, || {
                        imgui::Slider::new(im_str!("fps"), 1..=480).build(&ui, mut_fps);

                        ui.text(im_str!(
                            "render: {:?}",
                            profiler_render.get("frame_time").unwrap()
                        ));
                        ProgressBar::new(
                            profiler_render.get("frame_time").unwrap().as_secs_f32()
                                / (1.0 / *mut_fps as f32),
                        )
                        .build(&ui);
                        let mut others = profiler_render
                            .hm
                            .iter()
                            .filter(|(n, d)| *n != "frame_time")
                            .collect::<Vec<_>>();
                        others.sort_by_key(|e| e.0);
                        for (name, dur) in others.iter() {
                            let name: String = format!("{}                          ", name)
                                .chars()
                                .take(23)
                                .collect::<Vec<char>>()
                                .into_iter()
                                .collect();
                            ui.text(im_str!(" {}: {:?}", name, dur));
                        }

                        ui.separator();

                        ui.text(im_str!("logic: {:?}", profiler_logic.get("total").unwrap()));
                        ProgressBar::new(
                            profiler_logic.get("total").unwrap().as_millis() as f32 / 100.0,
                        )
                        .build(&ui);

                        let mut others = profiler_logic
                            .hm
                            .iter()
                            .filter(|(n, d)| *n != "total")
                            .collect::<Vec<_>>();
                        others.sort_by_key(|e| e.0);
                        for (name, dur) in others.iter() {
                            let name: String = format!("{}                          ", name)
                                .chars()
                                .take(23)
                                .collect::<Vec<char>>()
                                .into_iter()
                                .collect();
                            ui.text(im_str!(" {}: {:?}", name, dur));
                        }
                    });

                if fps_before != *mut_fps {
                    self.loop_helper = LoopHelper::builder().build_with_target_rate(*mut_fps as f64)
                }

                //Global info
                if let Some(global_info) = self.global_info {
                    let w = 300.0;
                    let h = 200.0;

                    let info_window = imgui::Window::new(im_str!("Global info"));

                    info_window
                        .size([w, h], imgui::Condition::FirstUseEver)
                        .position(
                            [self.gpu.sc_desc.width as f32 - w, 0.0],
                            imgui::Condition::Always,
                        )
                        .collapsed(true, imgui::Condition::FirstUseEver)
                        .resizable(true)
                        .movable(false)
                        .build(&ui, || {
                            ui.text(im_str!("{:#?}", global_info));
                        });
                }

                //Main menu
                match main_menu {
                    MainMode::Home => {
                        let w = 216.0;
                        let h = 324.0;
                        let home_window = imgui::Window::new(im_str!("Home"));

                        let mut next_mode = MainMode::Home;
                        let mut exit = false;
                        home_window
                            // .size([w, h], imgui::Condition::Always)
                            .position(
                                [(self.gpu.sc_desc.width as f32 - w) / 2.0, 100.0],
                                imgui::Condition::Always,
                            )
                            .title_bar(false)
                            .always_auto_resize(true)
                            .resizable(false)
                            .movable(false)
                            .scroll_bar(false)
                            .collapsible(false)
                            .build(&ui, || {
                                if ui.button(im_str!("Play"), [200.0_f32, 100.0]) {
                                    next_mode = MainMode::Play;
                                }
                                if ui.button(im_str!("Map Editor"), [200.0_f32, 100.0]) {
                                    next_mode = MainMode::MapEditor;
                                }
                                if ui.button(im_str!("Unit Editor"), [200.0_f32, 100.0]) {
                                    next_mode = MainMode::UnitEditor;
                                }
                                if ui.button(im_str!("Multiplayer"), [200.0_f32, 100.0]) {
                                    next_mode = MainMode::MultiplayerLobby;
                                }
                                if ui.button(im_str!("Exit"), [200.0_f32, 100.0]) {
                                    exit = true;
                                }
                            });

                        if exit {
                            self.sender_to_event_loop.send(EventLoopMsg::Stop).unwrap();
                        }
                        if self.main_menu != next_mode {
                            self.mailbox.push(RenderEvent::ChangeMode {
                                from: self.main_menu,
                                to: next_mode,
                            });
                            self.main_menu = next_mode;
                        }
                    }
                    MainMode::Play => {}
                    MainMode::MapEditor => {
                        self.game_state
                            .heightmap_editor
                            .draw_ui(&ui, &mut self.heightmap_gpu);
                    }
                    MainMode::UnitEditor => {
                        Self::draw_unit_editor_ui(
                            &ui,
                            &mut self.unit_editor,
                            &mut self.generic_gpu,
                        );
                    }
                    MainMode::MultiplayerLobby => {
                        let w = 216.0;
                        let h = 324.0;
                        let home_window = imgui::Window::new(im_str!("Multiplayer Lobby"));

                        let mut create_server = false;
                        let mut create_client = false;
                        let mut disconnect_server = false;
                        let mut disconnect_client = false;
                        let mut next_mode = MainMode::MultiplayerLobby;
                        if let Some(global_info) = self.global_info {
                            home_window
                                .size([w, h], imgui::Condition::Always)
                                .position(
                                    [
                                        (self.gpu.sc_desc.width as f32 - w) / 2.0,
                                        (self.gpu.sc_desc.height as f32 - h) / 2.0,
                                    ],
                                    imgui::Condition::Always,
                                )
                                .title_bar(false)
                                .resizable(false)
                                .movable(false)
                                .collapsible(false)
                                .build(&ui, || {
                                    if global_info.net_server.is_none()
                                        && global_info.net_client.is_none()
                                    {
                                        create_server =
                                            ui.button(im_str!("Start server"), [200.0_f32, 100.0]);
                                        create_client =
                                            ui.button(im_str!("Start client"), [200.0_f32, 100.0]);
                                    } else if global_info.net_server.is_some() {
                                        disconnect_server = ui.button(
                                            im_str!("Disconnect server"),
                                            [200.0_f32, 100.0],
                                        );
                                    } else if global_info.net_client.is_some() {
                                        disconnect_client = ui.button(
                                            im_str!("Disconnect client"),
                                            [200.0_f32, 100.0],
                                        );
                                    }

                                    if ui.button(im_str!("Back"), [200.0_f32, 100.0]) {
                                        next_mode = MainMode::Home;
                                    }
                                });
                        }

                        if create_server {
                            self.net_mode = NetMode::Server;
                            let e = client::FromClient::StartServer(client::StartServer {
                                bind: "127.0.0.1:4567".to_owned(),
                            });
                            let _ = self.sender_from_client_to_manager.try_send(e);
                        }

                        if create_client {
                            self.net_mode = NetMode::Client;
                            let e = client::FromClient::StartClient(client::StartClient {
                                bind: "127.0.0.1:4567".to_owned(),
                            });
                            let _ = self.sender_from_client_to_manager.try_send(e);
                        }
                        if disconnect_server {
                            self.net_mode = NetMode::Offline;
                            let e = client::FromClient::DisconnectServer;
                            let _ = self.sender_from_client_to_manager.try_send(e);
                        }
                        if disconnect_client {
                            self.net_mode = NetMode::Offline;
                            let e = client::FromClient::DisconnectClient;
                            let _ = self.sender_from_client_to_manager.try_send(e);
                        }

                        if self.main_menu != next_mode {
                            self.mailbox.push(RenderEvent::ChangeMode {
                                from: self.main_menu,
                                to: next_mode,
                            });
                            self.main_menu = next_mode;
                        }
                    }
                }
            }
            self.imgui_wrap
                .platform
                .prepare_render(&ui, &self.gpu.window);
        }
        self.profiler.mix("imgui_render", start.elapsed(), 20);

        let frame = &self.gpu.swap_chain.get_next_texture();
        let now = Instant::now();
        //Pass
        {
            log::trace!("begin_render_pass");
            let mut rpass = encoder_render.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.first_color_att_view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        },
                    },
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.position_att_view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color {
                            r: -1.0,
                            g: -1.0,
                            b: -1.0,
                            a: -1.0,
                        },
                    },
                ],
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
            for (path, generic_gpu_state) in self.generic_gpu.iter_mut() {
                match generic_gpu_state {
                    GenericGpuState::Ready(model_gpu) => {
                        model_gpu.render(&mut rpass, &self.bind_group);
                    }
                    _ => {}
                }
            }
            self.kbot_gpu.render(&mut rpass, &self.bind_group);
            self.kinematic_projectile_gpu
                .render(&mut rpass, &self.bind_group);
            self.arrow_gpu.render(&mut rpass, &self.bind_group);
        }

        //Transparent pass
        {
            log::trace!("begin_render_pass transparent");
            let mut rpass = encoder_render.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &self.first_color_att_view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Load,
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
                    depth_load_op: wgpu::LoadOp::Load,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
            });

            self.water_gpu.render(&mut rpass, &self.bind_group);
        }

        // Post pass
        {
            log::trace!("begin_post_render_pass");
            let mut rpass = encoder_render.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &self.first_color_att_view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Load,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    },
                }],

                depth_stencil_attachment: None,
            });

            self.postfx
                .render(&mut rpass, &self.gpu.device, &self.bind_group);
        }

        //Post fxaa pass
        {
            log::trace!("begin_post_render_pass");
            let mut rpass = encoder_render.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &self.secon_color_att_view,
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

                depth_stencil_attachment: None,
            });

            self.postfxaa
                .render(&mut rpass, &self.gpu.device, &self.bind_group);
        }

        //Custom Ui pass
        {
            log::trace!("begin_post_render_pass");
            let mut rpass = encoder_render.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &self.secon_color_att_view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Load,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 1.0,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    },
                }],

                depth_stencil_attachment: None,
            });

            self.health_bar.render(&mut rpass, &self.bind_group);
            self.unit_icon.render(&mut rpass, &self.bind_group);
            self.explosion_gpu.render(&mut rpass, &self.bind_group);
            self.line_gpu.render(&mut rpass, &self.bind_group);
        }

        //Copy on frame view
        {
            let mut rpass = encoder_render.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Load,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 1.0,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    },
                }],

                depth_stencil_attachment: None,
            });

            self.post_bicopy
                .render(&mut rpass, &self.gpu.device, &self.bind_group);
        }

        let render_pass_3d = now.elapsed();

        self.profiler.mix("render_pass_3d", render_pass_3d, 20);

        self.imgui_wrap
            .renderer
            .render(ui, &self.gpu.device, &mut encoder_render, &frame.view)
            .expect("Rendering failed");

        let start = Instant::now();
        self.gpu.queue.submit(&[encoder_render.finish()]);
        self.profiler
            .mix("device queue submit", start.elapsed(), 20);

        if let (true, Some(id), Some(mouse_world_pos)) = (
            self.input_state
                .mouse_trigger
                .contains(&winit::event::MouseButton::Right),
            self.game_state.my_player_id,
            self.game_state.mouse_world_pos,
        ) {
            let player_input = FrameEventFromPlayer::MoveOrder {
                id,
                selected: self.game_state.selected.clone(),
                mouse_world_pos,
            };

            log::info!("Move order from {}", id);

            let _ = self
                .sender_from_client_to_manager
                .try_send(client::FromClient::PlayerInput(player_input));
        }

        self.input_state.update();

        let tx = self.sender_to_client.clone();
        cursor_sample_position.map_read_async(0, 4 * 4, move |e: BufferMapAsyncResult<&[f32]>| {
            match e {
                Ok(e) => {
                    log::trace!("BufferMapAsyncResult callback");
                    let _ = tx.try_send(ToClient::MapReadAsyncMessage {
                        vec: e.data.to_vec(),
                        usage: "mouse_world_pos".to_owned(),
                    });
                }
                Err(_) => {}
            }
        });

        let tx = self.sender_to_client.clone();
        screen_center_sample_position.map_read_async(
            0,
            4 * 4,
            move |e: BufferMapAsyncResult<&[f32]>| match e {
                Ok(e) => {
                    log::trace!("BufferMapAsyncResult callback");
                    let _ = tx.try_send(ToClient::MapReadAsyncMessage {
                        vec: e.data.to_vec(),
                        usage: "screen_center_world_pos".to_owned(),
                    });
                }
                Err(_) => {}
            },
        );
    }
}

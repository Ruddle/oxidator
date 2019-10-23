use super::client::*;
use crate::frame;
use crate::frame::FrameEvent;
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
        log::trace!("render");

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

        let mailbox = self.mailbox.clone();
        self.mailbox.clear();

        for mail in mailbox {
            match mail {
                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::MapEditor,
                } => {
                    self.clear_from_play();
                    self.game_state.position = Point3::new(1024.0, 400.0, 1100.0);
                    self.game_state.dir = Vector3::new(0.0, 0.3, -1.0);
                }

                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::Play,
                } => {
                    self.clear_from_play();
                    self.game_state.position = Point3::new(200.0, 100.0, 50.0);
                    self.game_state.dir = Vector3::new(0.0, 0.3, -1.0);

                    let mut player_me = Player::new();

                    for i in (150..300).step_by(2) {
                        for j in (100..200).step_by(2) {
                            let m = mobile::KBot::new(Point3::new(i as f32, j as f32, 100.0));
                            player_me.kbots.insert(m.id);
                            self.game_state.kbots.insert(m.id, m);
                        }
                    }

                    let mut player_ennemy = Player::new();
                    player_ennemy.team = 1;

                    for i in (320..420).step_by(4) {
                        for j in (100..200).step_by(4) {
                            let m = mobile::KBot::new(Point3::new(i as f32, j as f32, 100.0));
                            player_ennemy.kbots.insert(m.id);
                            self.game_state.kbots.insert(m.id, m);
                        }
                    }

                    self.game_state.my_player_id = Some(player_me.id);
                    self.game_state.players.insert(player_me.id, player_me);
                    self.game_state
                        .players
                        .insert(player_ennemy.id, player_ennemy);

                    let replacer = FrameEvent::ReplaceFrame(frame::Frame {
                        number: 0,
                        players: self.game_state.players.clone(),
                        kbots: self.game_state.kbots.clone(),
                        kinematic_projectiles: self.game_state.kinematic_projectiles.clone(),
                        events: Vec::new(),
                        arrows: Vec::new(),
                        heightmap_phy: self.heightmap_gpu.phy.clone(),
                        complete: true,
                        frame_profiler: frame::FrameProfiler::new(),
                    });
                    let _ = self
                        .sender_from_client
                        .try_send(client::FromClient::Event(replacer));
                }

                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::Home,
                } => {
                    self.game_state.position = Point3::new(200.0, 100.0, 50.0);
                    self.game_state.dir = Vector3::new(0.0, 0.3, -1.0);
                    let replacer = FrameEvent::ReplaceFrame(frame::Frame::new());
                    let _ = self
                        .sender_from_client
                        .try_send(client::FromClient::Event(replacer));
                }
                _ => {}
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

        // Camera Movements
        if self.main_menu != MainMode::Home {
            use winit::event::VirtualKeyCode as Key;
            let key_pressed = &self.input_state.key_pressed;
            let on = |vkc| key_pressed.contains(&vkc);

            let mut offset = Vector3::new(0.0, 0.0, 0.0);
            let mut dir_offset = self.game_state.dir.clone();
            let mut new_dir = None;

            let camera_ground_height = self.heightmap_gpu.get_z(
                self.game_state
                    .position
                    .x
                    .max(0.0)
                    .min(self.heightmap_gpu.phy.width as f32 - 1.0),
                self.game_state
                    .position
                    .y
                    .max(0.0)
                    .min(self.heightmap_gpu.phy.height as f32 - 1.0),
            );
            let height_from_ground = self.game_state.position.z - camera_ground_height;
            let distance_camera_middle_screen = self
                .game_state
                .screen_center_world_pos
                .map(|scwp| (self.game_state.position.coords - scwp).magnitude())
                .unwrap_or(height_from_ground);
            let k = (if !on(Key::LShift) { 1.0 } else { 2.0 })
                * distance_camera_middle_screen.max(10.0);
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
                if let Some(screen_center_world_pos) = self.game_state.screen_center_world_pos {
                    if self.input_state.last_scroll != 0.0 {
                        let camera_to_center =
                            screen_center_world_pos - self.game_state.position.coords;

                        let distance = camera_to_center.norm();

                        let mut new_camera_to_center = camera_to_center.normalize();

                        if self.input_state.last_scroll > 0.0 {
                            new_camera_to_center.y += 1.0 * 0.30;
                        }
                        if self.input_state.last_scroll < 0.0 {
                            new_camera_to_center.z -= 1.0 * 0.30;
                        }
                        new_camera_to_center.x = 0.0;

                        new_camera_to_center = new_camera_to_center.normalize();
                        new_camera_to_center.y = new_camera_to_center.y.max(0.01);

                        new_dir = Some(new_camera_to_center);
                        let new_pos =
                            screen_center_world_pos - new_camera_to_center.normalize() * distance;
                        offset += (new_pos - self.game_state.position.coords) / delta_sim_sec;
                    }
                } else {
                    if self.input_state.last_scroll > 0.0 {
                        dir_offset.y += 1.0 / delta_sim_sec;
                    }
                    if self.input_state.last_scroll < 0.0 {
                        dir_offset.z -= 1.0 / delta_sim_sec;
                    }
                }
            } else {
                if let Some(mouse_world_pos) = self.game_state.mouse_world_pos {
                    let u = (mouse_world_pos - self.game_state.position.coords).normalize();
                    offset += self.input_state.last_scroll * u * k * 0.75 * 0.320 / delta_sim_sec;
                } else {
                    offset.z = -self.input_state.last_scroll * k * 0.75 * 0.20 / delta_sim_sec;
                }
            }

            self.input_state.last_scroll = 0.0;

            self.game_state.position += offset * delta_sim_sec;
            self.game_state.dir =
                (self.game_state.dir + dir_offset * 33.0 * delta_sim_sec).normalize();

            new_dir.map(|new_dir| {
                self.game_state.dir = new_dir;
            });

            self.game_state.position.z = self.game_state.position.z.max(camera_ground_height + 3.0);

            self.game_state.position_smooth += (self.game_state.position.coords
                - self.game_state.position_smooth.coords)
                * delta_sim_sec.min(0.033)
                * 15.0;

            self.game_state.dir_smooth += (self.game_state.dir - self.game_state.dir_smooth)
                * delta_sim_sec.min(0.033)
                * 15.0;
        }

        // self.phy_state.step();
        //Phy Drawing
        // {
        //     let cubes_t = self.phy_state.cubes_transform();
        //     let mut positions = Vec::with_capacity(cubes_t.len() * 16);
        //     for mat in cubes_t {
        //         positions.extend_from_slice(mat.as_slice())
        //     }

        //     self.cube_gpu
        //         .update_instance(&positions[..], &self.gpu.device);
        // }

        let (interp_duration, mobile_to_gpu_duration) = if self.main_menu == MainMode::Play {
            self.handle_play(delta_sim_sec)
        } else {
            (Duration::default(), Duration::default())
        };

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

        //Render
        let mut encoder_render = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        let heightmap_gpu_step_duration = time(|| {
            self.heightmap_gpu
                .step(&self.gpu.device, &mut encoder_render);
        });

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

        let ub_misc = self
            .gpu
            .device
            .create_buffer_mapped(8, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&[
                self.input_state.cursor_pos.0 as f32,
                self.input_state.cursor_pos.1 as f32,
                self.gpu.sc_desc.width as f32,
                self.gpu.sc_desc.height as f32,
                1.0 / self.gpu.sc_desc.width as f32,
                1.0 / self.gpu.sc_desc.height as f32,
                start_drag.0,
                start_drag.1,
            ]);

        encoder_render.copy_buffer_to_buffer(&ub_misc, 0, &self.ub_misc, 0, 8 * 4);

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

        let frame = &self.gpu.swap_chain.get_next_texture();

        now = Instant::now();
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
                            a: 0.0,
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
            self.cube_gpu.render(&mut rpass, &self.bind_group);
            self.kbot_gpu.render(&mut rpass, &self.bind_group);
            self.kinematic_projectile_gpu
                .render(&mut rpass, &self.bind_group);
            self.arrow_gpu.render(&mut rpass, &self.bind_group);
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

                depth_stencil_attachment: None,
            });

            self.postfxaa.render(
                &mut rpass,
                &self.gpu.device,
                &self.bind_group,
                &self.first_color_att_view,
            );
        }
        let us_3d_render_pass = now.elapsed();

        //Imgui
        {
            log::trace!("imgui render");
            self.imgui_wrap
                .platform
                .prepare_frame(self.imgui_wrap.imgui.io_mut(), &self.gpu.window)
                .expect("Failed to prepare frame");

            let ui: Ui = self.imgui_wrap.imgui.frame();

            let main_menu = &mut self.main_menu;

            {
                let mut_fps = &mut self.game_state.fps;
                let p = &self.game_state.frame_zero.frame_profiler;
                let stats_window = imgui::Window::new(im_str!("Statistics"));
                stats_window
                    .size([270.0, 260.0], imgui::Condition::FirstUseEver)
                    .position([3.0, 3.0], imgui::Condition::FirstUseEver)
                    .collapsed(true, imgui::Condition::FirstUseEver)
                    .resizable(true)
                    .movable(false)
                    .build(&ui, || {
                        imgui::Slider::new(im_str!("fps"), 1..=480).build(&ui, mut_fps);
                        ui.text(im_str!("Frametime: {}us", last_compute_time.as_micros()));
                        ui.text(im_str!(
                            " \" Capped: {}us",
                            last_compute_time_total.as_micros()
                        ));

                        ui.separator();
                        ui.text(im_str!("renderer:               {:?}", last_compute_time));
                        ui.text(im_str!("   interpolation:       {:?}", interp_duration));
                        ui.text(im_str!(
                            "   mobile_to_gpu:       {:?}",
                            mobile_to_gpu_duration
                        ));
                        ui.text(im_str!(
                            "   heightmap_editor:    {:?}",
                            heightmap_editor_duration
                        ));
                        ui.text(im_str!(
                            "   heightmap_step:      {:?}",
                            heightmap_gpu_step_duration
                        ));
                        ui.text(im_str!("   3d_render_pass:      {:?}", us_3d_render_pass));
                        ui.separator();

                        ui.text(im_str!(
                            "logic:                  {:?}",
                            p.get("total").unwrap()
                        ));

                        let mut others =
                            p.hm.iter()
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

                match main_menu {
                    MainMode::Home => {
                        let w = 216.0;
                        let h = 324.0;
                        let home_window = imgui::Window::new(im_str!("Home"));

                        let mut next_mode = MainMode::Home;
                        let mut exit = false;
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
                                if ui.button(im_str!("Play"), [200.0_f32, 100.0]) {
                                    next_mode = MainMode::Play;
                                }
                                if ui.button(im_str!("Map Editor"), [200.0_f32, 100.0]) {
                                    next_mode = MainMode::MapEditor;
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
                }

                // self.phy_state.draw_ui(&ui);
            }
            self.imgui_wrap
                .platform
                .prepare_render(&ui, &self.gpu.window);
            self.imgui_wrap
                .renderer
                .render(ui, &self.gpu.device, &mut encoder_render, &frame.view)
                .expect("Rendering failed");
        }

        self.gpu
            .device
            .get_queue()
            .submit(&[encoder_render.finish()]);

        if let Some(id) = self.game_state.my_player_id {
            let player_input = FrameEvent::PlayerInput {
                id,
                input_state: self.input_state.clone(),
                selected: self.game_state.selected.clone(),
                mouse_world_pos: self.game_state.mouse_world_pos,
            };

            let _ = self
                .sender_from_client
                .try_send(client::FromClient::Event(player_input));
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

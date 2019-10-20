use super::app::*;
use crate::*;
use imgui::*;
use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};

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
        let min_us = 1000000_u64 / self.input_state.fps;
        let min_wait_until_next_frame = std::time::Duration::from_micros(min_us - oversleep);
        if min_wait_until_next_frame > delta {
            std::thread::sleep(min_wait_until_next_frame - delta);
        }

        now = Instant::now();
        delta = now - self.game_state.last_frame;
        self.game_state.last_frame = now;
        let last_compute_time_total = delta.clone();
        let delta_sim_sec = last_compute_time_total.as_secs_f32();
        self.frame_count += 1;

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
                    for i in (200..250).step_by(7) {
                        for j in (100..150).step_by(7) {
                            let m = mobile::Mobile::new(Point3::new(i as f32, j as f32, 100.0));

                            self.game_state.mobiles.insert(m.id.clone(), m);
                        }
                    }
                }

                RenderEvent::ChangeMode {
                    from,
                    to: MainMode::Home,
                } => {
                    self.game_state.position = Point3::new(200.0, 100.0, 50.0);
                    self.game_state.dir = Vector3::new(0.0, 0.3, -1.0)
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
        {
            use winit::event::VirtualKeyCode as Key;
            let key_pressed = &self.input_state.key_pressed;
            let on = |vkc| key_pressed.contains(&vkc);

            let mut offset = Vector3::new(0.0, 0.0, 0.0);
            let mut rotation = self.game_state.dir.clone();

            let camera_ground_height = self.heightmap_gpu.get_z(
                self.game_state
                    .position
                    .x
                    .max(0.0)
                    .min(self.heightmap_gpu.width as f32 - 1.0),
                self.game_state
                    .position
                    .y
                    .max(0.0)
                    .min(self.heightmap_gpu.height as f32 - 1.0),
            );
            let height_from_ground = self.game_state.position.z - camera_ground_height;
            let k = (if !on(Key::LShift) { 1.0 } else { 2.0 }) * height_from_ground.max(10.0);
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
                if self.input_state.last_scroll > 0.0 {
                    rotation.y += 1.0
                }
                if self.input_state.last_scroll < 0.0 {
                    rotation.z -= 1.0
                }
            } else {
                offset.z = -self.input_state.last_scroll * k * 20.0;
            }

            self.input_state.last_scroll = 0.0;

            self.game_state.position += offset * delta_sim_sec;
            self.game_state.dir =
                (self.game_state.dir + rotation * 33.0 * delta_sim_sec).normalize();

            self.game_state.position.z = self.game_state.position.z.max(camera_ground_height + 3.0);

            self.game_state.position_smooth += (self.game_state.position.coords
                - self.game_state.position_smooth.coords)
                * delta_sim_sec.min(0.033)
                * 15.0;

            self.game_state.dir_smooth += (self.game_state.dir - self.game_state.dir_smooth)
                * delta_sim_sec.min(0.033)
                * 15.0;
        }

        self.phy_state.step();
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

        let (us_update_mobiles, us_mobile_to_gpu) = if self.main_menu == MainMode::Play {
            self.handle_play(delta_sim_sec)
        } else {
            (0, 0)
        };

        //Action
        now = Instant::now();
        if let MainMode::MapEditor = self.main_menu {
            if let Some(mouse_world_pos) = self.game_state.mouse_world_pos {
                self.game_state.heightmap_editor.handle_user_input(
                    &self.input_state.mouse_pressed,
                    &mouse_world_pos,
                    &mut self.heightmap_gpu,
                );
            }
        }
        let us_heightmap_editor = now.elapsed().as_micros();

        //Render
        let mut encoder_render = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        now = Instant::now();
        self.heightmap_gpu
            .step(&self.gpu.device, &mut encoder_render);
        let us_heightmap_step = now.elapsed().as_micros();

        let cursor_sample_position = self
            .gpu
            .device
            .create_buffer_mapped::<f32>(
                4,
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
            )
            .finish(); //.fill_from_slice(initial);

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
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
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
            self.mobile_gpu.render(&mut rpass, &self.bind_group);
        }

        //Post pass
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

            self.postfx.render(
                &mut rpass,
                &self.gpu.device,
                &self.bind_group,
                &self.position_att_view,
            );
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
        let us_3d_render_pass = now.elapsed().as_micros();

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
                let mut_fps = &mut self.input_state.fps;
                let debug_i1 = &mut self.input_state.debug_i1;
                let stats_window = imgui::Window::new(im_str!("Statistics"));
                stats_window
                    .size([400.0, 200.0], imgui::Condition::FirstUseEver)
                    .position([3.0, 3.0], imgui::Condition::FirstUseEver)
                    .collapsed(true, imgui::Condition::FirstUseEver)
                    .resizable(false)
                    .movable(false)
                    .build(&ui, || {
                        imgui::Slider::new(im_str!("fps"), 1..=480).build(&ui, mut_fps);
                        ui.text(im_str!("Frametime: {}us", last_compute_time.as_micros()));
                        ui.text(im_str!(
                            " \" Capped: {}us",
                            last_compute_time_total.as_micros()
                        ));

                        ui.text(im_str!("us_update_mobiles: {}us", us_update_mobiles));
                        ui.text(im_str!("us_mobile_to_gpu: {}us", us_mobile_to_gpu));
                        ui.text(im_str!("us_heightmap_editor: {}us", us_heightmap_editor));
                        ui.text(im_str!("us_heightmap_step: {}us", us_heightmap_step));
                        ui.text(im_str!("us_3d_render_pass: {}us", us_3d_render_pass));

                        if imgui::Slider::new(im_str!("debug_i1"), 1..=1000).build(&ui, debug_i1) {}
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

        self.input_state.update();

        let tx = self.sender_to_app.clone();
        cursor_sample_position.map_read_async(0, 4 * 4, move |e: BufferMapAsyncResult<&[f32]>| {
            match e {
                Ok(e) => {
                    log::trace!("BufferMapAsyncResult callback");
                    let _ = tx.try_send(AppMsg::MapReadAsyncMessage {
                        vec: e.data.to_vec(),
                    });
                }
                Err(_) => {}
            }
        });

        let _ = self.sender_to_app.try_send(AppMsg::Render);
    }
}

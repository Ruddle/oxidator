use super::client::*;
use crate::*;
use imgui::*;
use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use utils::*;

impl App {
    pub fn handle_play(&mut self, delta_sim_sec: f32, encoder: &mut wgpu::CommandEncoder) {
        let view_proj = camera::create_view_proj(
            self.gpu.sc_desc.width as f32 / self.gpu.sc_desc.height as f32,
            &self.game_state.position_smooth,
            &self.game_state.dir_smooth,
        );
        //Interpolate
        let interp_duration = time(|| {
            self.game_state.interpolate(&self.threadpool, &view_proj);
        });

        // Projecting on screen
        let projecting_screen = time(|| {
            if let Some(me) = self.game_state.my_player() {
                //Selection square
                if let input_state::Drag::End { x0, y0, x1, y1 } = self.input_state.drag {
                    let start_sel = std::time::Instant::now();
                    let min_x = (x0.min(x1) as f32 / self.gpu.sc_desc.width as f32) * 2.0 - 1.0;
                    let min_y = (y0.min(y1) as f32 / self.gpu.sc_desc.height as f32) * 2.0 - 1.0;
                    let max_x = (x0.max(x1) as f32 / self.gpu.sc_desc.width as f32) * 2.0 - 1.0;
                    let max_y = (y0.max(y1) as f32 / self.gpu.sc_desc.height as f32) * 2.0 - 1.0;
                    let selected: HashSet<IdValue> = self
                        .game_state
                        .in_screen
                        .iter()
                        .filter(|(id, e)| me.kbots.contains(id))
                        .filter(|(_, e)| e.x > min_x && e.x < max_x && e.y < max_y && e.y > min_y)
                        .map(|(i, _)| i.value)
                        .collect();

                    log::trace!("Selection took {}us", start_sel.elapsed().as_micros());

                    self.game_state.selected = selected;
                } else if self
                    .input_state
                    .mouse_release
                    .contains(&winit::event::MouseButton::Left)
                {
                    self.game_state.selected.clear();
                }
            }
        });

        //Upload to gpu
        let mobile_to_gpu_duration = time(|| {
            {
                //Kbot
                self.vertex_attr_buffer_f32.clear();

                for mobile in self.game_state.kbots.values().filter(|e| e.is_in_screen) {
                    let mat = mobile.trans.unwrap();
                    let is_selected = if self.game_state.selected.contains(&mobile.id.value) {
                        1.0
                    } else {
                        0.0
                    };
                    let team = mobile.team;

                    self.vertex_attr_buffer_f32
                        .extend_from_slice(mat.as_slice());
                    self.vertex_attr_buffer_f32.push(is_selected);
                    self.vertex_attr_buffer_f32.push(team as f32)
                }

                self.kbot_gpu
                    .update_instance_dirty(&self.vertex_attr_buffer_f32[..], &self.gpu.device);
            }
            //Kinematic Projectile
            self.vertex_attr_buffer_f32.clear();
            for mobile in self.game_state.kinematic_projectiles.values() {
                let mat = Matrix4::face_towards(
                    &mobile.positions.iter().next().unwrap(),
                    &(mobile.positions.iter().next().unwrap() + Vector3::new(1.0, 0.0, 0.0)),
                    &Vector3::new(0.0, 0.0, 1.0),
                );

                let is_selected = if self.game_state.selected.contains(&mobile.id.value) {
                    1.0
                } else {
                    0.0
                };

                let team = -1.0;

                self.vertex_attr_buffer_f32
                    .extend_from_slice(mat.as_slice());
                self.vertex_attr_buffer_f32.push(is_selected);
                self.vertex_attr_buffer_f32.push(team)
            }

            self.kinematic_projectile_gpu
                .update_instance_dirty(&self.vertex_attr_buffer_f32[..], &self.gpu.device);

            //Arrow
            self.vertex_attr_buffer_f32.clear();
            for arrow in self.game_state.frame_zero.arrows.iter() {
                let mat = Matrix4::face_towards(
                    &arrow.position,
                    &arrow.end,
                    &Vector3::new(0.0, 0.0, 1.0),
                );

                self.vertex_attr_buffer_f32
                    .extend_from_slice(mat.as_slice());
                self.vertex_attr_buffer_f32
                    .extend_from_slice(&arrow.color[..3]);
                self.vertex_attr_buffer_f32
                    .push((arrow.end.coords - arrow.position.coords).magnitude());
            }

            self.arrow_gpu
                .update_instance(&self.vertex_attr_buffer_f32[..], &self.gpu.device);

            //Unit life

            self.vertex_attr_buffer_f32.clear();
            for (id, _) in self.game_state.in_screen.iter() {
                if let Some(kbot) = self.game_state.kbots.get(id) {
                    let distance =
                        (self.game_state.position_smooth.coords - kbot.position.coords).magnitude();

                    let alpha_range = 10.0;
                    let max_dist = 100.0;
                    let alpha = (1.0 + (max_dist - distance) / alpha_range)
                        .min(1.0)
                        .max(0.0)
                        .powf(2.0);

                    let alpha_range = 50.0;
                    let size_factor = (0.3 + (max_dist - distance) / alpha_range)
                        .min(1.0)
                        .max(0.3)
                        .powf(1.0);

                    let life = kbot.life as f32 / kbot.max_life as f32;
                    if alpha > 0.0 && life < 1.0 {
                        let w = self.gpu.sc_desc.width as f32;
                        let h = self.gpu.sc_desc.height as f32;
                        let half_size = Vector2::new(20.0 / w, 3.0 / h) * size_factor;

                        // u is direction above kbot in camera space
                        // right cross camera_to_unit = u
                        let camera_to_unit =
                            kbot.position.coords - self.game_state.position_smooth.coords;
                        let right = Vector3::new(1.0, 0.0, 0.0);

                        let u = right.cross(&camera_to_unit).normalize();

                        let world_pos = kbot.position + u * kbot.radius * 1.5;
                        let r = view_proj * world_pos.to_homogeneous();
                        let r = r / r.w;

                        let offset = Vector2::new(r.x, r.y);
                        let min = offset - half_size;
                        let max = offset + half_size;
                        let life = kbot.life as f32 / kbot.max_life as f32;
                        self.vertex_attr_buffer_f32
                            .extend_from_slice(min.as_slice());
                        self.vertex_attr_buffer_f32
                            .extend_from_slice(max.as_slice());
                        self.vertex_attr_buffer_f32.push(life);
                        self.vertex_attr_buffer_f32.push(alpha);
                    }
                }
            }
            self.health_bar
                .update_instance(&self.vertex_attr_buffer_f32[..], &self.gpu.device);
        });

        self.profiler.mix("interp", interp_duration, 20);
        self.profiler
            .mix("mobile_to_gpu", mobile_to_gpu_duration, 20);
        self.profiler
            .mix("projecting_screen", projecting_screen, 20);
    }
}

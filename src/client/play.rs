use super::client::*;
use crate::*;
use imgui::*;
use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use utils::*;

impl App {
    pub fn handle_play(&mut self, delta_sim_sec: f32) -> (Duration, Duration) {
        //Interpolate
        let interp_duration = time(|| {
            self.game_state.interpolate();
        });

        //Selection square
        {
            if let input_state::Drag::End { x0, y0, x1, y1 } = self.input_state.drag {
                let min_x = (x0.min(x1) as f32 / self.gpu.sc_desc.width as f32) * 2.0 - 1.0;
                let min_y = (y0.min(y1) as f32 / self.gpu.sc_desc.height as f32) * 2.0 - 1.0;
                let max_x = (x0.max(x1) as f32 / self.gpu.sc_desc.width as f32) * 2.0 - 1.0;
                let max_y = (y0.max(y1) as f32 / self.gpu.sc_desc.height as f32) * 2.0 - 1.0;

                // Projecting on screen
                let view_proj = camera::create_view_proj(
                    self.gpu.sc_desc.width as f32 / self.gpu.sc_desc.height as f32,
                    &self.game_state.position_smooth,
                    &self.game_state.dir_smooth,
                );

                let me = self.game_state.my_player().unwrap();

                let start_proj = std::time::Instant::now();
                let projected = self
                    .game_state
                    .kbots
                    .iter()
                    .filter(|(id, e)| me.mobiles.contains(id))
                    .flat_map(|(id, e)| {
                        let p = e.position.to_homogeneous();
                        let r = view_proj * p;
                        //Keeping those of the clipped space in screen (-1 1, -1 1 , 0 1)
                        if r.z > 0.0 && r.x < r.w && r.x > -r.w && r.y < r.w && r.y > -r.w {
                            Some((id, Vector2::new(r.x / r.w, r.y / r.w)))
                        } else {
                            None
                        }
                    });

                let selected: HashSet<IdValue> = projected
                    .filter(|(_, e)| e.x > min_x && e.x < max_x && e.y < max_y && e.y > min_y)
                    .map(|(i, _)| i.value)
                    .collect();

                println!("Selection took {}us", start_proj.elapsed().as_micros());

                self.game_state.selected = selected;
            } else if self
                .input_state
                .mouse_release
                .contains(&winit::event::MouseButton::Left)
            {
                self.game_state.selected.clear();
            }
        }

        //Upload to gpu
        let mobile_to_gpu_duration = time(|| {
            //Kbot
            let mut positions = Vec::with_capacity(self.game_state.kbots.len() * 18);
            for mobile in self.game_state.kbots.values() {
                let mat = Matrix4::face_towards(
                    &mobile.position,
                    &(mobile.position + mobile.dir),
                    &Vector3::new(0.0, 0.0, 1.0),
                );

                let is_selected = if self.game_state.selected.contains(&mobile.id.value) {
                    1.0
                } else {
                    0.0
                };

                let team = self
                    .game_state
                    .players
                    .values()
                    .find(|e| e.mobiles.contains(&mobile.id))
                    .unwrap()
                    .team;

                positions.extend_from_slice(mat.as_slice());
                positions.push(is_selected);
                positions.push(team as f32)
            }

            self.kbot_gpu
                .update_instance(&positions[..], &self.gpu.device);

            //Kinematic Projectile
            let mut positions =
                Vec::with_capacity(self.game_state.kinematic_projectiles.len() * 18);
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

                positions.extend_from_slice(mat.as_slice());
                positions.push(is_selected);
                positions.push(team)
            }

            self.kinematic_projectile_gpu
                .update_instance(&positions[..], &self.gpu.device);

            //Arrow
            let mut positions = Vec::with_capacity(self.game_state.frame_zero.arrows.len() * 20);
            for arrow in self.game_state.frame_zero.arrows.iter() {
                let mat = Matrix4::face_towards(
                    &arrow.position,
                    &arrow.end,
                    &Vector3::new(0.0, 0.0, 1.0),
                );

                positions.extend_from_slice(mat.as_slice());
                positions.extend_from_slice(&arrow.color[..3]);
                positions.push((arrow.end.coords - arrow.position.coords).magnitude());
            }

            self.arrow_gpu
                .update_instance(&positions[..], &self.gpu.device);
        });

        (interp_duration, mobile_to_gpu_duration)
    }
}

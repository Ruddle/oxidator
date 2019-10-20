use super::app::*;
use crate::*;
use imgui::*;
use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};
use std::collections::{HashMap, HashSet};

use utils::time;

impl App {
    pub fn handle_play(&mut self, delta_sim_sec: f32) -> (u128, u128) {
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

                let start_proj = std::time::Instant::now();
                let projected = self.game_state.mobiles.iter().flat_map(|(id, e)| {
                    let p = e.position.to_homogeneous();
                    let r = view_proj * p;
                    //Keeping those of the clipped space in screen (-1 1, -1 1 , 0 1)
                    if r.z > 0.0 && r.x < r.w && r.x > -r.w && r.y < r.w && r.y > -r.w {
                        Some((id, Vector2::new(r.x / r.w, r.y / r.w)))
                    } else {
                        None
                    }
                });

                let selected: HashSet<String> = projected
                    .filter(|(_, e)| e.x > min_x && e.x < max_x && e.y < max_y && e.y > min_y)
                    .map(|(i, _)| i.clone())
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

        //Mobile update target
        group_behavior::Group::update_mobile_target(
            &self.input_state.mouse_trigger,
            self.game_state.mouse_world_pos,
            &self.game_state.selected,
            &mut self.game_state.mobiles,
        );

        //Mobile update
        let us_update_mobiles = time(|| {
            group_behavior::Group::update_mobiles(
                delta_sim_sec,
                &mut self.game_state.mobiles,
                &self.heightmap_gpu,
            )
        });

        let us_mobile_to_gpu = time(|| {
            let mut positions = Vec::with_capacity(self.game_state.mobiles.len() * 17);
            for mobile in self.game_state.mobiles.values() {
                let mat = Matrix4::face_towards(
                    &mobile.position,
                    &(mobile.position + mobile.dir),
                    &Vector3::new(0.0, 0.0, 1.0),
                );

                let is_selected = if self.game_state.selected.contains(&mobile.id) {
                    1.0
                } else {
                    0.0
                };

                positions.extend_from_slice(mat.as_slice());
                positions.push(is_selected);
            }

            self.mobile_gpu
                .update_instance(&positions[..], &self.gpu.device);
        });

        (us_update_mobiles, us_mobile_to_gpu)
    }
}

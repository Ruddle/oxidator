use super::client::*;
use crate::*;

impl App {
    pub fn clear_gpu_instance_and_game_state(&mut self) {
        self.game_state.players.clear();
        self.game_state.my_player_id = None;
        self.game_state.kbots.clear();
        self.game_state.selected.clear();
        self.game_state.explosions.clear();
        self.game_state.kinematic_projectiles_cache.clear();
        self.unit_editor.root.children.clear();
        self.kbot_gpu.update_instance_dirty(&[], &self.gpu.device);
        self.health_bar.update_instance(&[], &self.gpu.device);
        self.unit_icon.update_instance(&[], &self.gpu.device);
        self.explosion_gpu.update_instance(&[], &self.gpu.device);
        for (_, generic_gpu_state) in self.generic_gpu.iter_mut() {
            match generic_gpu_state {
                GenericGpuState::Ready(model_gpu) => {
                    model_gpu.update_instance_dirty(&[], &self.gpu.device)
                }
                _ => {}
            }
        }
        self.kinematic_projectile_gpu
            .update_instance_dirty(&[], &self.gpu.device);
    }

    pub fn visit_part_tree(
        part_tree: &unit_editor::PartTree,
        view_proj: &Matrix4<f32>,
        generic_gpu: &mut HashMap<PathBuf, GenericGpuState>,
    ) {
        for c in part_tree.children.iter() {
            if let Some(dmodel) = &c.dmodel {
                match generic_gpu.get_mut(&dmodel.model_path) {
                    Some(GenericGpuState::Ready(generic_cpu)) => {
                        let buf = &mut generic_cpu.instance_attr_cpu_buf;
                        let display_model = &dmodel;

                        let mat = Matrix4::face_towards(
                            &display_model.position,
                            &(display_model.position + display_model.dir),
                            &Vector3::new(0.0, 0.0, 1.0),
                        );

                        buf.extend_from_slice(mat.as_slice());
                        buf.push(0.0);
                        buf.push(0.0);
                    }
                    _ => {}
                }
            }

            Self::visit_part_tree(c, view_proj, generic_gpu);
        }
    }

    pub fn upload_to_gpu(&mut self, view_proj: &Matrix4<f32>, encoder: &mut wgpu::CommandEncoder) {
        //Upload to gpu
        let upload_to_gpu_duration = time(|| {
            let unit_icon_distance = self.game_state.unit_icon_distance;

            //generic_gpu
            {
                for (path, model_gpu) in self.generic_gpu.iter_mut() {
                    match model_gpu {
                        GenericGpuState::Ready(model_gpu) => {
                            model_gpu.instance_attr_cpu_buf.clear();
                        }
                        _ => {}
                    }
                }

                Self::visit_part_tree(&mut self.unit_editor.root, view_proj, &mut self.generic_gpu);

                for (path, model_gpu) in self.generic_gpu.iter_mut() {
                    match model_gpu {
                        GenericGpuState::Ready(model_gpu) => {
                            model_gpu.update_instance_dirty_own_buffer(&self.gpu.device);
                        }
                        _ => {}
                    }
                }

                // for (path, model_gpu) in self.generic_gpu.iter_mut() {
                //     match model_gpu {
                //         GenericGpuState::Ready(model_gpu) => {
                //             self.vertex_attr_buffer_f32.clear();
                //             for display_model in self
                //                 .unit_editor
                //                 .parts
                //                 .iter()
                //                 .filter(|e| e.model_path == *path)
                //             {
                //                 let mat = Matrix4::face_towards(
                //                     &display_model.position,
                //                     &(display_model.position + display_model.dir),
                //                     &Vector3::new(0.0, 0.0, 1.0),
                //                 );

                //                 self.vertex_attr_buffer_f32
                //                     .extend_from_slice(mat.as_slice());
                //                 self.vertex_attr_buffer_f32.push(0.0);
                //                 self.vertex_attr_buffer_f32.push(0.0);
                //             }

                //             model_gpu.update_instance_dirty(
                //                 &self.vertex_attr_buffer_f32[..],
                //                 &self.gpu.device,
                //             );
                //         }
                //         _ => {
                //             // log::warn!("ModelGpu {:?} not ready", path);
                //         }
                //     }
                // }
            }

            //Kbot
            {
                self.vertex_attr_buffer_f32.clear();

                for mobile in self
                    .game_state
                    .kbots
                    .iter()
                    .filter(|e| e.is_in_screen && e.distance_to_camera < unit_icon_distance)
                {
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
            for mobile in self.game_state.kinematic_projectiles.iter() {
                let mat = Matrix4::face_towards(
                    &mobile,
                    &(mobile + Vector3::new(1.0, 0.0, 0.0)),
                    &Vector3::new(0.0, 0.0, 1.0),
                );

                let is_selected = 0.0;

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
            for kbot in self
                .game_state
                .kbots
                .iter()
                .filter(|e| e.is_in_screen && e.distance_to_camera < unit_icon_distance)
            {
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
            self.health_bar
                .update_instance(&self.vertex_attr_buffer_f32[..], &self.gpu.device);

            //Icon
            self.vertex_attr_buffer_f32.clear();
            for kbot in self
                .game_state
                .kbots
                .iter()
                .filter(|e| e.is_in_screen && e.distance_to_camera >= unit_icon_distance)
            {
                self.vertex_attr_buffer_f32
                    .extend_from_slice(kbot.screen_pos.as_slice());
                //TODO f(distance) instead of 20.0
                let size = ((1.0 / (kbot.distance_to_camera / unit_icon_distance)) * 15.0).max(4.0);
                self.vertex_attr_buffer_f32.push(size);

                let is_selected = self.game_state.selected.contains(&kbot.id.value);
                let team = if is_selected { -1.0 } else { kbot.team as f32 };
                self.vertex_attr_buffer_f32.push(team);
            }
            self.unit_icon
                .update_instance(&self.vertex_attr_buffer_f32[..], &self.gpu.device);

            //Explosions
            self.vertex_attr_buffer_f32.clear();
            for explosion in self.game_state.explosions.iter() {
                let screen_pos = view_proj * explosion.position.to_homogeneous();
                if screen_pos.z > 0.0
                    && screen_pos.x > -screen_pos.w
                    && screen_pos.x < screen_pos.w
                    && screen_pos.y > -screen_pos.w
                    && screen_pos.y < screen_pos.w
                {
                    let distance =
                        (self.game_state.position_smooth.coords - explosion.position.coords).norm();
                    self.vertex_attr_buffer_f32
                        .push(screen_pos.x / screen_pos.w);
                    self.vertex_attr_buffer_f32
                        .push(screen_pos.y / screen_pos.w);
                    self.vertex_attr_buffer_f32
                        .push(explosion.size * 2500.0 / distance);

                    self.vertex_attr_buffer_f32.push(
                        (self.game_state.server_sec - explosion.born_sec)
                            / (explosion.death_sec - explosion.born_sec),
                    );
                    self.vertex_attr_buffer_f32.push(explosion.seed);
                }
            }
            self.explosion_gpu
                .update_instance(&self.vertex_attr_buffer_f32[..], &self.gpu.device);
        });
        self.profiler
            .mix("upload_to_gpu", upload_to_gpu_duration, 20);
    }
}

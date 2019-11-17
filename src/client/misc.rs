use super::client::*;
use crate::*;
use unit_part_gpu::*;
impl App {
    pub fn clear_gpu_instance_and_game_state(&mut self) {
        self.game_state.players.clear();
        self.game_state.my_player_id = None;
        self.game_state.kbots.clear();
        self.game_state.selected.clear();
        self.game_state.explosions.clear();
        self.game_state.kinematic_projectiles_cache.clear();
        // self.unit_editor.root.children.clear();
        self.kbot_gpu.update_instance_dirty(&[], &self.gpu.device);
        self.health_bar.update_instance(&[], &self.gpu.device);
        self.unit_icon.update_instance(&[], &self.gpu.device);
        self.explosion_gpu.update_instance(&[], &self.gpu.device);
        for (model_gpu_state) in self.unit_part_gpu.states.iter_mut() {
            match model_gpu_state {
                ModelGpuState::Ready(model_gpu) => {
                    model_gpu.update_instance_dirty(&[], &self.gpu.device)
                }
                _ => {}
            }
        }
        self.kinematic_projectile_gpu
            .update_instance_dirty(&[], &self.gpu.device);
    }

    pub fn visit_part_tree(
        part_tree: &unit::PartTree,
        root_trans: &Matrix4<f32>,
        unit_part_gpu: &mut UnitPartGpu,
        selected: f32,
        team: f32,
        weapon0_dir: Vector3<f32>,
        wheel0_angle: f32,
    ) {
        for c in part_tree.children.iter() {
            if let Some(placed_mesh) = &c.placed_mesh {
                let display_model = &placed_mesh;

                let combined = match &c.joint {
                    unit::Joint::Fix => root_trans * c.parent_to_self,
                    unit::Joint::AimWeapon0 => {
                        let comb = root_trans * c.parent_to_self;

                        utils::face_towards_dir(
                            &Vector3::new(comb[12], comb[13], comb[14]),
                            &weapon0_dir,
                            &Vector3::new(0.0, 0.0, 1.0),
                        )
                    }
                    unit::Joint::Wheel0 => {
                        let comb = root_trans * c.parent_to_self;

                        comb * utils::face_towards_dir(
                            &Vector3::new(0.0, 0.0, 0.0),
                            &Vector3::new(0.0, 1.0, 0.0),
                            &Vector3::new(f32::cos(wheel0_angle), 0.0, f32::sin(wheel0_angle)),
                        )
                    }
                };

                let for_display = combined * display_model.trans;
                // log::warn!(
                //     "root {:?}\nlocal {:?}\ncombined {:?}\n",
                //     root_trans,
                //     mat,
                //     combined
                // );

                //TODO fix performance : HALF OF TIME IS IN GET_MUT
                match unit_part_gpu.get_mut(placed_mesh.mesh_index) {
                    //}  get_mut(&placed_mesh.mesh_path) {
                    ModelGpuState::Ready(generic_cpu) => {
                        let buf = &mut generic_cpu.instance_attr_cpu_buf;

                        let isometry: Isometry3<f32> = unsafe {
                            na::convert_unchecked::<Matrix4<f32>, Isometry3<f32>>(for_display)
                        };
                        let euler = isometry.rotation.euler_angles();
                        buf.push(for_display[12]);
                        buf.push(for_display[13]);
                        buf.push(for_display[14]);
                        buf.push(euler.0);
                        buf.push(euler.1);
                        buf.push(euler.2);
                        buf.push(selected);
                        buf.push(team);
                    }
                    _ => {}
                }
                Self::visit_part_tree(
                    c,
                    &combined,
                    unit_part_gpu,
                    selected,
                    team,
                    weapon0_dir,
                    wheel0_angle,
                );
            } else {
                Self::visit_part_tree(
                    c,
                    root_trans,
                    unit_part_gpu,
                    selected,
                    team,
                    weapon0_dir,
                    wheel0_angle,
                );
            }
        }
    }

    pub fn upload_to_gpu(&mut self, view_proj: &Matrix4<f32>, encoder: &mut wgpu::CommandEncoder) {
        //Upload to gpu
        let upload_to_gpu_duration = time(|| {
            let unit_icon_distance = self.game_state.unit_icon_distance;

            //generic_gpu
            {
                for model_gpu in self.unit_part_gpu.states.iter_mut() {
                    match model_gpu {
                        ModelGpuState::Ready(model_gpu) => {
                            model_gpu.instance_attr_cpu_buf.clear();
                        }
                        _ => {}
                    }
                }

                let identity = utils::face_towards_dir(
                    &Vector3::new(300.0_f32, 100.0, 0.50),
                    &Vector3::new(1.0, 0.0, 0.0),
                    &Vector3::new(0.0, 0.0, 1.0),
                ); //Matrix4::identity();

                let t = self.game_state.start_time.elapsed().as_secs_f32();
                if self.main_menu == MainMode::UnitEditor {
                    Self::visit_part_tree(
                        &self.unit_editor.botdef.part_tree,
                        &identity,
                        &mut self.unit_part_gpu,
                        0.0,
                        0.0,
                        Vector3::new(f32::cos(t), f32::sin(t), f32::sin(t / 5.0) * 0.1).normalize(),
                        (t * 2.0),
                    );
                }

                //Kbot
                {
                    for (mobile, client_kbot) in
                        self.game_state.kbots.iter_mut().filter(|e| {
                            e.1.is_in_screen && e.1.distance_to_camera < unit_icon_distance
                        })
                    {
                        let mat = client_kbot.trans.unwrap();
                        let is_selected = if self.game_state.selected.contains(&mobile.id.value) {
                            1.0
                        } else {
                            0.0
                        };
                        let team = mobile.team;

                        if let Some(botdef) =
                            self.game_state.frame_zero.bot_defs.get(&mobile.botdef_id)
                        {
                            Self::visit_part_tree(
                                &botdef.part_tree,
                                &mat,
                                &mut self.unit_part_gpu,
                                is_selected,
                                team as f32,
                                client_kbot.weapon0_dir,
                                client_kbot.wheel0_angle,
                            );
                        }
                    }
                }

                for model_gpu in self.unit_part_gpu.states.iter_mut() {
                    match model_gpu {
                        ModelGpuState::Ready(model_gpu) => {
                            model_gpu.update_instance_dirty_own_buffer(&self.gpu.device);
                        }
                        _ => {}
                    }
                }
            }

            // //Kbot
            // {
            //     self.vertex_attr_buffer_f32.clear();

            //     for mobile in self
            //         .game_state
            //         .kbots
            //         .iter()
            //         .filter(|e| e.is_in_screen && e.distance_to_camera < unit_icon_distance)
            //     {
            //         let mat = mobile.trans.unwrap();
            //         let is_selected = if self.game_state.selected.contains(&mobile.id.value) {
            //             1.0
            //         } else {
            //             0.0
            //         };
            //         let team = mobile.team;

            //         self.vertex_attr_buffer_f32
            //             .extend_from_slice(mat.as_slice());
            //         self.vertex_attr_buffer_f32.push(is_selected);
            //         self.vertex_attr_buffer_f32.push(team as f32)
            //     }

            //     self.kbot_gpu
            //         .update_instance_dirty(&self.vertex_attr_buffer_f32[..], &self.gpu.device);
            // }
            //Kinematic Projectile
            self.vertex_attr_buffer_f32.clear();
            for mobile in self.game_state.kinematic_projectiles.iter() {
                let mat = utils::face_towards_dir(
                    &mobile.coords,
                    &(Vector3::new(1.0, 0.0, 0.0)),
                    &Vector3::new(0.0, 0.0, 1.0),
                );

                let is_selected = 0.0;

                let team = -1.0;

                let isometry: Isometry3<f32> =
                    unsafe { na::convert_unchecked::<Matrix4<f32>, Isometry3<f32>>(mat) };
                let euler = isometry.rotation.euler_angles();

                self.vertex_attr_buffer_f32.push(mat[12]);
                self.vertex_attr_buffer_f32.push(mat[13]);
                self.vertex_attr_buffer_f32.push(mat[14]);
                self.vertex_attr_buffer_f32.push(euler.0);
                self.vertex_attr_buffer_f32.push(euler.1);
                self.vertex_attr_buffer_f32.push(euler.2);
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
            for (kbot, client_kbot) in self
                .game_state
                .kbots
                .iter()
                .filter(|e| e.1.is_in_screen && e.1.distance_to_camera < unit_icon_distance)
            {
                let distance = (self.game_state.position_smooth.coords
                    - client_kbot.position.coords)
                    .magnitude();

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

                let botdef = self
                    .game_state
                    .frame_zero
                    .bot_defs
                    .get(&kbot.botdef_id)
                    .unwrap();
                let life = kbot.life as f32 / botdef.max_life as f32;
                if alpha > 0.0 && life < 1.0 {
                    let w = self.gpu.sc_desc.width as f32;
                    let h = self.gpu.sc_desc.height as f32;
                    let half_size = Vector2::new(20.0 / w, 3.0 / h) * size_factor;

                    // u is direction above kbot in camera space
                    // right cross camera_to_unit = u
                    let camera_to_unit =
                        client_kbot.position.coords - self.game_state.position_smooth.coords;
                    let right = Vector3::new(1.0, 0.0, 0.0);

                    let u = right.cross(&camera_to_unit).normalize();

                    let world_pos = client_kbot.position + u * botdef.radius * 1.5;
                    let r = view_proj * world_pos.to_homogeneous();
                    let r = r / r.w;

                    let offset = Vector2::new(r.x, r.y);
                    let min = offset - half_size;
                    let max = offset + half_size;
                    let life = kbot.life as f32 / botdef.max_life as f32;
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
            for (kbot, client_kbot) in self
                .game_state
                .kbots
                .iter()
                .filter(|e| e.1.is_in_screen && e.1.distance_to_camera >= unit_icon_distance)
            {
                self.vertex_attr_buffer_f32
                    .extend_from_slice(client_kbot.screen_pos.as_slice());
                //TODO f(distance) instead of 20.0
                let size =
                    ((1.0 / (client_kbot.distance_to_camera / unit_icon_distance)) * 15.0).max(4.0);
                self.vertex_attr_buffer_f32.push(size);

                let is_selected = self.game_state.selected.contains(&kbot.id.value);
                let team = if is_selected { -1.0 } else { kbot.team as f32 };
                self.vertex_attr_buffer_f32.push(team);
            }
            self.unit_icon
                .update_instance(&self.vertex_attr_buffer_f32[..], &self.gpu.device);

            //Line
            self.vertex_attr_buffer_f32.clear();
            {
                let mut count = 0;
                let see_all_order = self
                    .input_state
                    .key_pressed
                    .contains(&winit::event::VirtualKeyCode::LShift);
                {
                    for (kbot, client_kbot) in self.game_state.kbots.iter() {
                        if see_all_order || self.game_state.selected.contains(&kbot.id.value) {
                            if let Some(target) = kbot.target {
                                let min = view_proj * client_kbot.position.to_homogeneous();
                                let max = view_proj * target.to_homogeneous();
                                if (min.z > 0.0
                                    && min.x > -min.w
                                    && min.x < min.w
                                    && min.y > -min.w
                                    && min.y < min.w)
                                    || (max.z > 0.0
                                        && max.x > -max.w
                                        && max.x < max.w
                                        && max.y > -max.w
                                        && max.y < max.w)
                                {
                                    count += 1;
                                    self.vertex_attr_buffer_f32.push(min.x / min.w);
                                    self.vertex_attr_buffer_f32.push(min.y / min.w);
                                    self.vertex_attr_buffer_f32.push(max.x / max.w);
                                    self.vertex_attr_buffer_f32.push(max.y / max.w);
                                    self.vertex_attr_buffer_f32.push(0.0);
                                    self.vertex_attr_buffer_f32.push(0.0);
                                }
                            }
                        }
                    }
                    for i in (0..self.vertex_attr_buffer_f32.len()).step_by(6) {
                        self.vertex_attr_buffer_f32[i + 5] = count as f32;
                    }
                }
            }
            self.line_gpu
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

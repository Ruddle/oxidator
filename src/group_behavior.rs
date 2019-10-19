use crate::heightmap_gpu;
use crate::mobile;
use crate::utils;
use na::{Matrix4, Point3, Vector2, Vector3};
use std::collections::{HashMap, HashSet};
pub struct Group {}

impl Group {
    pub fn update_mobile_target(
        mouse_triggered: &HashSet<winit::event::MouseButton>,
        mouse_world_pos: Option<Vector3<f32>>,
        selected: &HashSet<String>,
        mobiles: &mut HashMap<String, mobile::Mobile>,
    ) {
        match (
            mouse_triggered.contains(&winit::event::MouseButton::Right),
            mouse_world_pos,
        ) {
            (true, Some(mouse_pos_world)) => {
                let selected_count = selected.len();
                let formation_w = (selected_count as f32).sqrt().ceil() as i32;

                let mut spot = Vec::<Vector3<f32>>::new();
                for i in 0..formation_w {
                    for j in 0..formation_w {
                        spot.push(
                            mouse_pos_world
                                + Vector3::new(
                                    i as f32 - formation_w as f32 / 2.0,
                                    j as f32 - formation_w as f32 / 2.0,
                                    0.0,
                                ) * 4.0,
                        )
                    }
                }

                let mut center = Vector3::new(0.0, 0.0, 0.0);
                let mut tap = 0.0;

                let mut id_to_pos = Vec::new();
                for s in selected.iter() {
                    if let Some(mobile) = mobiles.get(s) {
                        id_to_pos.push((mobile.id.clone(), mobile.position.coords));
                        center += mobile.position.coords;
                        tap += 1.0;
                    }
                }
                center /= tap;

                let axis = (mouse_pos_world - center).normalize();

                let mut projected_spot: Vec<_> = spot
                    .iter()
                    .enumerate()
                    .map(|(index, v)| (index, v.dot(&axis)))
                    .collect();

                projected_spot.sort_by(|(_, proj), (_, proj2)| {
                    if proj > proj2 {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                });

                let mut id_to_proj: Vec<_> = id_to_pos
                    .iter()
                    .map(|(index, v)| (index, v.dot(&axis)))
                    .collect();

                id_to_proj.sort_by(|(_, proj), (_, proj2)| {
                    if proj > proj2 {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                });

                for ((id, _), (spot_id, _)) in id_to_proj.iter().zip(&projected_spot[..]) {
                    if let Some(mobile) = mobiles.get_mut(*id) {
                        println!("New order for {}", mobile.id.clone());
                        mobile.target = Some(Point3::<f32>::from(spot[*spot_id]));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn update_mobiles(
        dt: f32,
        mobiles: &mut HashMap<String, mobile::Mobile>,
        heightmap_gpu: &heightmap_gpu::HeightmapGpu,
    ) {
        let cell_size = 16;
        let grid_w = (heightmap_gpu.width / cell_size) as usize;
        let grid_h = (heightmap_gpu.height / cell_size) as usize;
        let mut grid = vec![HashSet::<String>::new(); grid_w * grid_h];

        let grid_pos = |mobile: &mobile::Mobile| -> usize {
            let (x, y) = (mobile.position.x, mobile.position.y);
            (x as usize / cell_size as usize) as usize
                + (y as usize / cell_size as usize) as usize * grid_w
        };

        for (id, mobile) in mobiles.iter() {
            grid[grid_pos(mobile)].insert(id.clone());
        }

        let mobiles2 = mobiles.clone();

        {
            {
                for (id, mobile) in mobiles.iter_mut() {
                    let grid_pos = grid_pos(mobile);

                    let mut neighbors_id = grid[grid_pos].clone();
                    neighbors_id.remove(id);
                    for cell in &[
                        -1_i32 - grid_w as i32,
                        -(grid_w as i32),
                        1 - grid_w as i32,
                        -1,
                        1,
                        -1 + grid_w as i32,
                        grid_w as i32,
                        1 + grid_w as i32,
                    ] {
                        let cell_index = cell + grid_pos as i32;
                        if cell_index >= 0 && (cell_index as usize) < grid_w * grid_h {
                            neighbors_id.union(&grid[cell_index as usize]);
                        }
                    }

                    let mut collision_avoid_dir = Vector3::new(0.0_f32, 0.0, 0.0);
                    let mut collision_avoid_priority = 0.0_f32;

                    let mut neighbor_dir = Vector3::new(0.0_f32, 0.0, 0.0);
                    let mut neighbor_dir_priority = 0.0_f32;

                    let mut dir = Vector3::new(0.0, 0.0, 0.0);

                    if neighbors_id.len() == 0 {
                    } else {
                        let frame_prediction = 15.0;
                        let mut nearest = None;
                        let mut dist_min = None;
                        for neighbor_id in neighbors_id.iter() {
                            let neighbor = mobiles2.get(neighbor_id).unwrap();
                            let dist = (neighbor.position + neighbor.speed * frame_prediction
                                - &mobile.position
                                - mobile.speed * frame_prediction)
                                .norm_squared();

                            let is_better = match dist_min {
                                None => true,
                                Some(dist_min) => dist_min > dist,
                            };

                            if is_better {
                                dist_min = Some(dist);
                                nearest = Some(neighbor);
                            }
                        }

                        let dist_min = dist_min.unwrap().sqrt();
                        let nearest = nearest.unwrap();

                        if dist_min < 4.0 {
                            let closeness = (mobile.position.coords
                                + mobile.speed * frame_prediction
                                - nearest.position.coords
                                - nearest.speed * frame_prediction)
                                .magnitude();

                            collision_avoid_priority = ((4.0 - closeness) / 4.0).max(0.0).min(0.8);

                            collision_avoid_dir = if mobile.speed.dot(&nearest.speed) < 0.0 {
                                let u =
                                    (mobile.position.coords - nearest.position.coords).normalize();

                                let v = Vector3::<f32>::new(u.y, -u.x, u.z).normalize();
                                let w = Vector3::<f32>::new(-u.y, u.x, u.z).normalize();

                                if v.dot(&mobile.speed) > w.dot(&mobile.speed) {
                                    v
                                } else {
                                    w
                                }
                            } else {
                                let him_to_me =
                                    (mobile.position.coords - nearest.position.coords).normalize();
                                him_to_me
                            };

                            let speed_closeness = mobile.speed.dot(&nearest.speed);
                            neighbor_dir_priority =
                                if speed_closeness > 0.0 && nearest.speed.norm() > 0.1 {
                                    speed_closeness.max(0.2).min(1.0 - collision_avoid_priority)
                                } else {
                                    0.0
                                };

                            neighbor_dir = nearest
                                .speed
                                .try_normalize(0.001)
                                .unwrap_or(Vector3::new(0.0, 0.0, 0.0));
                        }
                    }

                    let mut dir_intensity = 0.0;

                    if let Some(target) = mobile.target {
                        let to_target = target.coords - mobile.position.coords;
                        let to_target_distance = to_target.norm();
                        let will_to_go_target = if to_target_distance > 0.5 {
                            (to_target_distance / 2.0).min(1.0)
                        } else {
                            0.0
                        };

                        let target_dir = to_target.normalize();

                        let available =
                            (1.0 - collision_avoid_priority - neighbor_dir_priority).max(0.0);

                        let target_prio = available.min(will_to_go_target);

                        let dir = target_dir * target_prio
                            + collision_avoid_dir * collision_avoid_priority
                            + neighbor_dir * neighbor_dir_priority;

                        dir_intensity =
                            target_prio + collision_avoid_priority + neighbor_dir_priority;

                        mobile.dir = mobile.dir * 0.95 + dir * 0.05;

                        if will_to_go_target < 0.01 {
                            mobile.target = None;
                        }
                    }

                    mobile.speed = (mobile.speed + mobile.dir * 0.08 * dir_intensity) * 0.5;

                    mobile.position += mobile.speed;

                    mobile.position.z =
                        heightmap_gpu.get_z_linear(mobile.position.x, mobile.position.y) + 0.5;
                }
            }
        }
    }
}

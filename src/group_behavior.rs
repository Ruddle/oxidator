use crate::heightmap_gpu;
use crate::mobile;
use crate::utils;
use na::{Matrix4, Point3, Vector2, Vector3};
use std::collections::{HashMap, HashSet};
pub struct Group {}

impl Group {
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

                    let mut dir = Vector3::new(0.0, 0.0, 0.0);

                    if neighbors_id.len() == 0 {
                    } else {
                        let frame_prediction = 30.0;
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

                        if dist_min < 10.0 {
                            let opposite = (mobile.position.coords
                                + mobile.speed * frame_prediction
                                - nearest.position.coords
                                - nearest.speed * frame_prediction)
                                .normalize();

                            let same_target = if let Some(d) = nearest.speed.try_normalize(0.001) {
                                d * 0.5
                            } else {
                                Vector3::new(0.0, 0.0, 0.0)
                            };
                            dir = 0.1 * (10.0 - dist_min) * (opposite + same_target);
                            if dist_min < 2.0 {
                                dir = opposite;
                            }
                        }
                    }

                    let target = mobile
                        .target
                        .map(|e| e.coords)
                        .unwrap_or(mobile.position.coords);
                    let mut to_target = target - mobile.position.coords;

                    let mut to_target_l = (to_target.norm() - 1.0).max(0.0);
                    if to_target_l > 1.0 {
                        to_target = to_target.normalize();
                        to_target_l = 1.0;
                    }

                    let dir = if let Some(d) = (dir + to_target * 0.5).try_normalize(0.001) {
                        d
                    } else {
                        Vector3::new(0.0, 0.0, 0.0)
                    };

                    mobile.dir = (mobile.dir * 0.97 + dir * 0.03).normalize();

                    mobile.speed = (mobile.speed + mobile.dir * 0.08 * to_target_l) * 0.5;

                    mobile.position += mobile.speed;

                    mobile.position.z =
                        heightmap_gpu.get_z_linear(mobile.position.x, mobile.position.y) + 0.5;
                }
            }
        }
    }
}

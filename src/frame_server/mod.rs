use crate::frame::*;

use crate::botdef;
use crate::heightmap_phy;
use crate::mobile::*;
use crate::utils::*;
use crossbeam_channel::{Receiver, Sender};
use na::{Matrix4, Point3, Vector2, Vector3};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub enum ToFrameServer {
    DataToComputeNextFrame(DataToComputeNextFrame),
}

pub enum FromFrameServer {
    NewFrame(Frame),
}

pub struct FrameServerCache {
    pub grid: Vec<Vec<Id<KBot>>>,
    pub small_grid: Vec<Vec<Id<KBot>>>,
    pub heightmap_phy: Option<heightmap_phy::HeightmapPhy>,
}

impl FrameServerCache {
    pub fn spawn(
        r_to_frame_server: Receiver<ToFrameServer>,
        s_from_frame_server: Sender<FromFrameServer>,
    ) -> () {
        let _ = std::thread::Builder::new()
            .name("frame_server".to_string())
            .spawn(move || {
                let mut fsc = FrameServerCache::new();
                for msg in r_to_frame_server.iter() {
                    match msg {
                        ToFrameServer::DataToComputeNextFrame(DataToComputeNextFrame {
                            old_frame,
                            events,
                        }) => {
                            let next_frame = fsc.next_frame(old_frame, events);
                            // let dur = utils::time(|| {
                            //     use flate2::write::ZlibEncoder;
                            //     use flate2::Compression;
                            //     use std::io::prelude::*;
                            //     let mut e = ZlibEncoder::new(Vec::new(), Compression::new(1));
                            //     e.write_all(&vec);
                            //     let compressed_bytes = e.finish().unwrap();
                            //     log::info!("Compressed is {} bytes", compressed_bytes.len());
                            // });
                            // log::info!("compression took {:?}", dur);
                            let _ = s_from_frame_server.send(FromFrameServer::NewFrame(next_frame));
                        }
                    }
                }
            });
    }

    pub fn new() -> Self {
        FrameServerCache {
            grid: Vec::new(),
            small_grid: Vec::new(),
            heightmap_phy: None,
        }
    }

    pub fn next_frame(&mut self, old_frame: Frame, events: Vec<FrameEventFromPlayer>) -> Frame {
        let mut frame_profiler = ProfilerMap::new();
        let start = std::time::Instant::now();
        log::trace!("Received frame {} to compute next frame", old_frame.number);

        log::trace!("Event {}", events.len());

        let mut replacer = None;
        for event in events.iter() {
            match event {
                FrameEventFromPlayer::ReplaceFrame(frame) => {
                    self.heightmap_phy = frame.heightmap_phy.clone();
                    replacer = Some(frame.clone());
                    log::trace!("Replacing frame");
                }
                _ => {}
            }
        }

        let mut frame = replacer.unwrap_or(old_frame);
        frame.number += 1;
        frame.kbots_dead.clear();
        frame.heightmap_phy = None;
        frame.explosions.clear();
        frame.kinematic_projectiles_birth.clear();
        frame.kinematic_projectiles_dead.clear();

        for event in events {
            match event {
                FrameEventFromPlayer::MoveOrder {
                    id,
                    selected,
                    mouse_world_pos,
                } => {
                    update_mobile_target(mouse_world_pos, &selected, &mut frame.kbots);
                }
                _ => {}
            }
        }

        frame_profiler.add("1 handle_events", start.elapsed());

        let mut arrows = Vec::new();

        let start_update_units = Instant::now();

        if let Some(heightmap) = &self.heightmap_phy {
            update_units(
                &mut frame_profiler,
                &mut frame.kbots,
                &mut frame.kbots_dead,
                &mut frame.kinematic_projectiles_dead,
                &mut frame.kinematic_projectiles_birth,
                &mut frame.kinematic_projectiles,
                heightmap,
                &mut arrows,
                frame.number,
                &frame.players,
                &mut self.grid,
                &mut self.small_grid,
                &mut frame.explosions,
                &frame.bot_defs,
            );
        }
        frame_profiler.add("0 update_units", start_update_units.elapsed());
        frame_profiler.add("total", start.elapsed());
        Frame {
            number: frame.number,
            frame_profiler,
            arrows,
            ..frame
        }
    }
}

pub fn update_mobile_target(
    mouse_world_pos: Vector3<f32>,
    selected: &HashSet<IdValue>,
    kbots: &mut HashMap<Id<KBot>, KBot>,
) {
    let selected_count = selected.len();
    let formation_w = (selected_count as f32).sqrt().ceil() as i32;

    let mut spot = Vec::<Vector3<f32>>::new();
    for i in 0..formation_w {
        for j in 0..formation_w {
            spot.push(
                mouse_world_pos
                    + Vector3::new(
                        i as f32 + 0.5 - formation_w as f32 / 2.0,
                        j as f32 + 0.5 - formation_w as f32 / 2.0,
                        0.0,
                    ) * 4.0,
            )
        }
    }

    let mut center = Vector3::new(0.0, 0.0, 0.0);
    let mut tap = 0.0;

    let mut id_to_pos = Vec::new();
    for &s in selected.iter() {
        if let Some(mobile) = kbots.get(&Id::new(s)) {
            id_to_pos.push((mobile.id, mobile.position.coords));
            center += mobile.position.coords;
            tap += 1.0;
        }
    }
    center /= tap;

    let axis = (mouse_world_pos - center).normalize();

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
        if let Some(mobile) = kbots.get_mut(id) {
            log::trace!("New order for {}", mobile.id);
            mobile.target = Some(Point3::<f32>::from(spot[*spot_id]));
        }
    }
}

pub fn update_units(
    frame_profiler: &mut ProfilerMap,
    kbots: &mut HashMap<Id<KBot>, KBot>,
    kbots_dead: &mut HashSet<Id<KBot>>,
    kinematic_projectiles_dead: &mut Vec<Id<KinematicProjectile>>,
    kinematic_projectiles_birth: &mut Vec<KinematicProjectile>,
    kinematic_projectiles: &mut HashMap<Id<KinematicProjectile>, KinematicProjectile>,
    heightmap_phy: &heightmap_phy::HeightmapPhy,
    arrows: &mut Vec<Arrow>,
    frame_count: i32,
    players: &HashMap<Id<Player>, Player>,
    grid: &mut Vec<Vec<Id<KBot>>>,
    small_grid: &mut Vec<Vec<Id<KBot>>>,
    explosions: &mut Vec<ExplosionEvent>,
    bot_defs: &HashMap<Id<botdef::BotDef>, botdef::BotDef>,
) {
    let start = std::time::Instant::now();
    let cell_size = 4;
    let grid_w = (heightmap_phy.width / cell_size) as usize;
    let grid_h = (heightmap_phy.height / cell_size) as usize;

    if grid.len() != grid_w * grid_h {
        std::mem::replace(grid, vec![Vec::<Id<KBot>>::new(); grid_w * grid_h]);
    } else {
        for zone in grid.iter_mut() {
            zone.clear();
        }
    }

    let grid_pos = |mobile: &KBot| -> usize {
        let (x, y) = (mobile.position.x, mobile.position.y);
        (x as usize / cell_size as usize) as usize
            + (y as usize / cell_size as usize) as usize * grid_w
    };

    for (&id, mobile) in kbots.iter() {
        let gp = grid_pos(mobile);
        grid[gp].push(id);

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
            let cell_index = cell + gp as i32;
            if cell_index >= 0 && (cell_index as usize) < grid_w * grid_h {
                grid[cell_index as usize].push(id);
            }
        }
    }

    frame_profiler.add("01  grid", start.elapsed());

    //AABB for kbot and proj
    {
        let start = std::time::Instant::now();
        let cell_size = 4;
        let grid_w = (heightmap_phy.width / cell_size) as usize;
        let grid_h = (heightmap_phy.height / cell_size) as usize;

        if small_grid.len() != grid_w * grid_h {
            std::mem::replace(small_grid, vec![Vec::<Id<KBot>>::new(); grid_w * grid_h]);
        } else {
            for zone in small_grid.iter_mut() {
                zone.clear();
            }
        }

        fn index_aabb(
            position: Vector3<f32>,
            radius: f32,
            cell_size: usize,
            grid_w: usize,
        ) -> Vec<usize> {
            let mut indices = Vec::new();
            let min_x = (position.x - radius * 1.0).floor() as usize;
            let max_x = (position.x + radius * 1.0).ceil() as usize;
            let min_y = (position.y - radius * 1.0).floor() as usize;
            let max_y = (position.y + radius * 1.0).ceil() as usize;

            let min_x = min_x / cell_size;
            let max_x = (max_x + 1) / cell_size;
            let min_y = min_y / cell_size;
            let max_y = (max_y + 1) / cell_size;

            for i in (min_x..=max_x).step_by(cell_size) {
                for j in (min_y..=max_y).step_by(cell_size) {
                    // println!("INSERTION {} {} {}", i, j, id);
                    indices.push(i + j * grid_w);
                }
            }
            indices
        }

        for (id, kbot) in kbots.iter() {
            let radius = bot_defs.get(&kbot.botdef_id).unwrap().radius;
            for index in index_aabb(kbot.position.coords, radius, cell_size, grid_w).iter() {
                small_grid[*index].push(*id);
            }
        }

        frame_profiler.add("03  small_grid", start.elapsed());

        let start = std::time::Instant::now();
        //Projectile move compute
        {
            for proj in kinematic_projectiles.values_mut() {
                let current_pos = proj.position_at(frame_count - 1);
                let next_pos = proj.position_at(frame_count);

                {
                    //Slowly interpolate to not miss collisions
                    let step_size = proj.radius * 1.0;
                    let ul = next_pos.coords - current_pos.coords;
                    let distance_to_travel = ul.magnitude();
                    let u = ul / distance_to_travel;

                    let mut current_interp = current_pos.coords.clone();
                    let count = (distance_to_travel / step_size).floor() as usize;
                    'interp: for n in 0..=count {
                        current_interp += u * step_size;
                        if n == count {
                            current_interp = next_pos.coords;
                        }

                        //Checking collision with current_interp
                        let indices = index_aabb(current_interp, proj.radius, cell_size, grid_w);

                        let kbots_in_proximity: HashSet<_> = indices
                            .iter()
                            .map(|index| small_grid[*index].clone())
                            .flatten()
                            .collect();
                        // &small_grid_kbot[index];

                        'bot_test: for kbot_id in kbots_in_proximity.iter() {
                            let kbot = kbots.get_mut(kbot_id).unwrap();
                            let distance_to_target =
                                (kbot.position.coords - current_interp).magnitude();

                            // println!("Distance {}", distance_to_target);
                            let kbot_radius = bot_defs.get(&kbot.botdef_id).unwrap().radius;
                            if distance_to_target < (kbot_radius + proj.radius) {
                                //Colission between Kbot and projectile
                                kbot.life = (kbot.life - 10).max(0);
                                proj.death_frame = frame_count;
                                explosions.push(ExplosionEvent {
                                    position: Point3::from(current_interp),
                                    size: 0.5,
                                    life_time: 0.8,
                                });
                                break 'interp;
                            }
                        }
                    }
                }

                if proj.death_frame == frame_count {
                    kinematic_projectiles_dead.push(proj.id);
                }
            }

            for r in kinematic_projectiles_dead.iter() {
                kinematic_projectiles.remove(&r);
            }
        }
        frame_profiler.add("04  proj move", start.elapsed());
    }

    //Projectile fire compute
    {
        let teams: HashSet<_> = players.values().map(|p| p.team).collect();

        let start = std::time::Instant::now();
        //TODO cache

        let mut id_to_team: HashMap<Id<KBot>, u8> = HashMap::with_capacity(kbots.len());

        for team in teams.iter() {
            let team_players: Vec<_> = players.values().filter(|e| &e.team == team).collect();
            for p in team_players.iter() {
                for kbot in p.kbots.iter() {
                    id_to_team.insert(*kbot, *team);
                }
            }
        }

        frame_profiler.add("05  id_to_team", start.elapsed());

        let start = std::time::Instant::now();
        struct Shot {
            bot: Id<KBot>,
            target: Vector3<f32>,
        };

        let mut shots = Vec::new();

        for (me, me_kbot) in kbots.iter() {
            let grid_pos = grid_pos(me_kbot);

            let my_team = id_to_team.get(me).unwrap();

            // let ennemies_in_cell = &team_to_ennemy_grid.get(my_team).unwrap()[grid_pos];

            let mut ennemies_in_cell: Vec<Id<KBot>> = grid[grid_pos].clone();
            let to_remove = ennemies_in_cell.iter().position(|e| e == me).unwrap();
            ennemies_in_cell.remove(to_remove);

            let can_shoot =// *my_team == 0&&
                 frame_count - me_kbot.frame_last_shot > me_kbot.reload_frame_count;
            if can_shoot {
                //We choose the first ennemy in the cell, we could sort by distance or something else here
                'meloop: for potential_ennemy in ennemies_in_cell {
                    if id_to_team.get(&potential_ennemy).unwrap() != my_team {
                        let ennemy_kbot = kbots.get(&potential_ennemy).unwrap();
                        if (ennemy_kbot.position.coords - me_kbot.position.coords).magnitude() < 6.0
                        {
                            shots.push(Shot {
                                bot: *me,
                                target: ennemy_kbot.position.coords,
                            });
                            break 'meloop;
                        }
                    }
                }
            }
        }

        for shot in shots.iter() {
            let kbot = kbots.get_mut(&shot.bot).unwrap();
            let dir = (shot.target - kbot.position.coords).normalize();

            kbot.weapon0_dir = dir;
            kbot.frame_last_shot = frame_count;
            let kbot_radius = bot_defs.get(&kbot.botdef_id).unwrap().radius;
            let proj = KinematicProjectile {
                id: rand_id(),
                birth_frame: frame_count,
                death_frame: frame_count + 6,
                position_at_birth: kbot.position + dir * kbot_radius * 1.0,
                speed_per_frame_at_birth: dir * 2.0 + Vector3::new(0.0, 0.0, 0.2),
                accel_per_frame: Vector3::new(0.0, 0.0, -0.08),
                radius: 0.25,
                position_cache: Vec::new(),
                speed_cache: Vec::new(),
            };
            kinematic_projectiles_birth.push(proj.clone());
            kinematic_projectiles.insert(proj.id, proj);
        }
        frame_profiler.add("07  kbot_fire", start.elapsed());
    }

    let start = std::time::Instant::now();
    let mobiles2 = kbots.clone();
    //Movement compute

    for (id, mobile) in kbots.iter_mut() {
        if mobile.speed.magnitude_squared() > 0.001 || mobile.target.is_some() || !mobile.grounded {
            let botdef = bot_defs.get(&mobile.botdef_id).unwrap();
            let grid_pos = grid_pos(mobile);
            let mut neighbors_id: Vec<Id<KBot>> = grid[grid_pos].clone();
            let to_remove = neighbors_id.iter().position(|e| e == id).unwrap();
            neighbors_id.remove(to_remove);

            let mut collision_avoid_dir = Vector2::new(0.0_f32, 0.0);
            let mut collision_avoid_priority = 0.0_f32;

            let mut neighbor_dir = Vector2::new(0.0_f32, 0.0);
            let mut neighbor_dir_priority = 0.0_f32;

            if neighbors_id.len() == 0 {
            } else {
                let frame_prediction = 1.0;
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
                    let closeness = (mobile.position.coords + mobile.speed * frame_prediction
                        - nearest.position.coords
                        - nearest.speed * frame_prediction)
                        .xy()
                        .magnitude();

                    collision_avoid_priority = ((4.0 - closeness) / 4.0).max(0.0).min(0.5);

                    collision_avoid_dir = if nearest.speed.xy().norm_squared() < 0.01
                        || mobile.speed.xy().dot(&nearest.speed.xy()) < 0.0
                    {
                        let u = (mobile.position.coords - nearest.position.coords)
                            .xy()
                            .normalize();

                        let v = Vector2::<f32>::new(u.y, -u.x).normalize();
                        let w = Vector2::<f32>::new(-u.y, u.x).normalize();

                        if v.dot(&mobile.speed.xy()) > w.dot(&mobile.speed.xy()) {
                            v
                        } else {
                            w
                        }
                    } else {
                        let him_to_me = (mobile.position.coords - nearest.position.coords)
                            .xy()
                            .normalize();
                        him_to_me
                    };

                    // arrows.push(Arrow {
                    //     position: mobile.position,
                    //     color: [collision_avoid_priority, 0.0, 0.0, 0.0],
                    //     end: mobile.position
                    //         + Vector3::new(
                    //             collision_avoid_dir.x * 2.0,
                    //             collision_avoid_dir.y * 2.0,
                    //             0.0,
                    //         ),
                    // });

                    let speed_closeness = mobile.speed.xy().dot(&nearest.speed.xy());
                    neighbor_dir_priority = if speed_closeness > 0.0 && nearest.speed.norm() > 0.1 {
                        speed_closeness
                            .max(0.0)
                            .min((1.0 - collision_avoid_priority) * 0.0)
                    } else {
                        0.0
                    };

                    neighbor_dir = nearest
                        .speed
                        .xy()
                        .try_normalize(0.001)
                        .unwrap_or(Vector2::new(0.0, 0.0));

                    // arrows.push(Arrow {
                    //     position: mobile.position,
                    //     color: [0.0, neighbor_dir_priority, 0.0, 0.0],
                    //     end: mobile.position
                    //         + Vector3::new(neighbor_dir.x * 2.0, neighbor_dir.y * 2.0, 0.0),
                    // });
                }
            }

            let mut dir_intensity = 0.0;

            if let Some(target) = mobile.target {
                let to_target = (target.coords - (mobile.position.coords + mobile.speed)).xy();
                let to_target_distance = to_target.norm();
                let will_to_go_target = if to_target_distance > 1.0 {
                    (to_target_distance / 2.0).min(1.0)
                } else {
                    0.0
                };

                let target_dir = to_target.normalize();
                let available = (1.0 - collision_avoid_priority - neighbor_dir_priority).max(0.0);
                let target_prio = available.min(will_to_go_target);
                let dir = target_dir * target_prio
                    + collision_avoid_dir * collision_avoid_priority
                    + neighbor_dir * neighbor_dir_priority;

                dir_intensity = target_prio + collision_avoid_priority + neighbor_dir_priority;

                // mobile.dir = mobile.dir * 0.50 + Vector3::new(dir.x, dir.y, 0.0) * 0.5;

                //Clamp in cone
                let wanted_angle: Angle = dir.into();
                let current_angle = mobile.angle;

                fn clamp_abs(x: f32, max_abs: f32) -> f32 {
                    let sign = x.signum();
                    sign * (x.abs().min(max_abs))
                }

                let diff = (wanted_angle - (current_angle + mobile.angular_velocity.into())).rad;

                mobile.angular_velocity = clamp_abs(
                    mobile.angular_velocity + clamp_abs(diff, botdef.turn_accel),
                    botdef.max_turn_rate,
                );

                let new_angle = current_angle + mobile.angular_velocity.into();
                // current_angle.clamp_around(wanted_angle, mobile.angular_velocity.into());
                mobile.angle = new_angle;
                let dir: Vector2<f32> = new_angle.into();
                mobile.dir = Vector3::new(dir.x, dir.y, 0.0);

                if will_to_go_target < 0.01 {
                    mobile.target = None;
                }
            }

            //TODO drift factor ?
            //drift = 1 (adherence = 0)
            // mobile.speed = mobile.speed + mobile.dir * botdef.accel * dir_intensity;
            //drift = 0 (adherence = 1)

            if mobile.target != None {
                mobile.speed =
                    mobile.dir * (botdef.accel * dir_intensity + mobile.speed.magnitude());
            } else {
                {
                    mobile.speed =
                        mobile.dir * (-botdef.break_accel + mobile.speed.magnitude()).max(0.0);
                }
            }

            let speed = mobile.speed.magnitude();
            if speed > botdef.max_speed {
                mobile.speed /= speed / botdef.max_speed;
            }

            mobile.position += mobile.speed;
            mobile.position.x = mobile
                .position
                .x
                .max(0.0)
                .min(heightmap_phy.width as f32 - 1.0);
            mobile.position.y = mobile
                .position
                .y
                .max(0.0)
                .min(heightmap_phy.height as f32 - 1.0);
            mobile.position.z = heightmap_phy.z_linear(mobile.position.x, mobile.position.y) + 0.5;
            mobile.grounded = true;
            mobile.up = heightmap_phy.normal(mobile.position.x, mobile.position.y);

            mobile.weapon0_dir = (mobile.weapon0_dir + mobile.dir).normalize();
        }
    }
    frame_profiler.add("02  movement", start.elapsed());

    //Remove dead kbot
    for (id, kbot) in kbots.iter() {
        if kbot.life <= 0 {
            kbots_dead.insert(*id);

            explosions.push(ExplosionEvent {
                position: Point3::from(kbot.position),
                size: 1.0,
                life_time: 1.2,
            });
        }
    }

    for id in kbots_dead.iter() {
        kbots.remove(id);
    }
}

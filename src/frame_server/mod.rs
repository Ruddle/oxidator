use crate::frame::*;

use crate::botdef;
use crate::heightmap_phy;
use crate::mobile::*;
use crate::utils::*;
use crossbeam_channel::{Receiver, Sender};
use fnv::{FnvHashMap, FnvHashSet};
use na::{Matrix4, Point3, Vector2, Vector3};
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

        //TODO order event by player then by type before doing any effect. This step should be deterministic
        for event in events {
            match event {
                FrameEventFromPlayer::MoveOrder {
                    id,
                    selected,
                    mouse_world_pos,
                } => {
                    //TODO Validate selected are owned by id
                    update_mobile_target(mouse_world_pos, &selected, &mut frame.kbots);
                }
                FrameEventFromPlayer::ConOrder {
                    id,
                    selected,
                    mouse_world_pos,
                    botdef_id,
                } => {
                    //TODO Validate selected are owned by id && botdef_id is constructable by at least 1 selected

                    let botdef = frame.bot_defs.get(&botdef_id).unwrap();
                    let mut m = KBot::new(Point3::from(mouse_world_pos), botdef, id);
                    m.team = frame.players.get(&id).unwrap().team;
                    m.con_completed = std::f32::MIN_POSITIVE;
                    m.life = 1;

                    for selected_raw_id in &selected {
                        for kbot in frame.kbots.get_mut(selected_raw_id) {
                            kbot.current_command = Command::Build(m.id.clone())
                        }
                    }

                    let player = frame.players.get_mut(&id).unwrap();
                    player.kbots.insert(m.id);
                    frame.kbots.insert(m.id, m);
                }

                FrameEventFromPlayer::RepairOrder {
                    id,
                    selected,
                    to_repair,
                } => {
                    for selected_raw_id in &selected {
                        for kbot in frame.kbots.get_mut(selected_raw_id) {
                            kbot.current_command = Command::Repair(to_repair)
                        }
                    }
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
                &mut frame.players,
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
    selected: &FnvHashSet<Id<KBot>>,
    kbots: &mut FnvHashMap<Id<KBot>, KBot>,
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
        if let Some(mobile) = kbots.get(&s) {
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
            mobile.move_target = Some(Point3::<f32>::from(spot[*spot_id]));
            mobile.current_command = Command::None;
        }
    }
}

pub fn update_units(
    frame_profiler: &mut ProfilerMap,
    kbots: &mut FnvHashMap<Id<KBot>, KBot>,
    kbots_dead: &mut FnvHashSet<Id<KBot>>,
    kinematic_projectiles_dead: &mut Vec<Id<KinematicProjectile>>,
    kinematic_projectiles_birth: &mut Vec<KinematicProjectile>,
    kinematic_projectiles: &mut FnvHashMap<Id<KinematicProjectile>, KinematicProjectile>,
    heightmap_phy: &heightmap_phy::HeightmapPhy,
    arrows: &mut Vec<Arrow>,
    frame_count: i32,
    players: &mut FnvHashMap<Id<Player>, Player>,
    grid: &mut Vec<Vec<Id<KBot>>>,
    small_grid: &mut Vec<Vec<Id<KBot>>>,
    explosions: &mut Vec<ExplosionEvent>,
    bot_defs: &FnvHashMap<Id<botdef::BotDef>, botdef::BotDef>,
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

                        let kbots_in_proximity: FnvHashSet<_> = indices
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
        let teams: FnvHashSet<_> = players.values().map(|p| p.team).collect();

        let start = std::time::Instant::now();
        //TODO cache

        let mut id_to_team: FnvHashMap<Id<KBot>, u8> =
            FnvHashMap::with_capacity_and_hasher(kbots.len(), Default::default());

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
            if me_kbot.con_completed == 1.0 {
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
                    //TODO Configurable strategy
                    'meloop: for potential_ennemy in ennemies_in_cell {
                        if id_to_team.get(&potential_ennemy).unwrap() != my_team {
                            let ennemy_kbot = kbots.get(&potential_ennemy).unwrap();
                            if (ennemy_kbot.position.coords - me_kbot.position.coords).magnitude()
                                < 6.0
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
                position_at_birth: kbot.position + dir * (kbot_radius + 0.25 + 0.01),
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

    struct BuildPart {
        amount: f64,
        repair: bool,
        player: Id<Player>,
        from: Id<KBot>,
        to: Id<KBot>,
    }
    let mut build_throughputs = Vec::new();
    //Build compute
    for (id, mobile) in kbots.iter_mut() {
        if mobile.con_completed >= 1.0 {
            // Look at current_command, change move_target if necessary
            match mobile.current_command {
                Command::Build(to_build) => match mobiles2.get(&to_build) {
                    Some(to_build) => {
                        if to_build.con_completed < 1.0 {
                            let dist =
                                (to_build.position.coords - mobile.position.coords).magnitude();
                            let botdef = bot_defs.get(&mobile.botdef_id).unwrap();
                            if dist <= botdef.build_dist {
                                mobile.move_target = None;
                                build_throughputs.push(BuildPart {
                                    amount: botdef.build_power as f64,
                                    repair: false,
                                    player: mobile.player_id,
                                    from: *id,
                                    to: to_build.id,
                                })
                            } else {
                                mobile.move_target = Some(to_build.position);
                            }
                        } else {
                            mobile.current_command = Command::None;
                            mobile.move_target = None;
                        }
                    }
                    None => {}
                },
                Command::Repair(to_build) => match mobiles2.get(&to_build) {
                    Some(to_build) => {
                        let botdef_of_to_build = bot_defs.get(&to_build.botdef_id).unwrap();
                        if to_build.life < botdef_of_to_build.max_life
                            || to_build.con_completed < 1.0
                        {
                            let dist =
                                (to_build.position.coords - mobile.position.coords).magnitude();
                            let botdef = bot_defs.get(&mobile.botdef_id).unwrap();
                            if dist <= botdef.build_dist {
                                mobile.move_target = None;
                                build_throughputs.push(BuildPart {
                                    amount: botdef.build_power as f64,
                                    repair: to_build.con_completed >= 1.0,
                                    player: mobile.player_id,
                                    from: *id,
                                    to: to_build.id,
                                })
                            } else {
                                mobile.move_target = Some(to_build.position);
                            }
                        } else {
                            mobile.current_command = Command::None;
                            mobile.move_target = None;
                        }
                    }
                    None => {}
                },
                _ => {}
            }
        }
    }

    //Compute resource usage for each player
    struct ResourceUsage {
        metal: f64,
        energy: f64,
    }
    let mut resources_usage = FnvHashMap::<Id<Player>, ResourceUsage>::default();

    for BuildPart {
        amount,
        to,
        repair,
        from,
        player,
    } in build_throughputs.iter()
    {
        let stat = resources_usage.entry(*player).or_insert(ResourceUsage {
            metal: 0.0,
            energy: 0.0,
        });
        *stat = ResourceUsage {
            metal: stat.metal + if *repair { 0.0 } else { *amount as f64 },
            energy: 0.0,
        };
    }
    //Compute what proportion of usage is usable without negative stock
    struct ResourceUsagePropMax {
        metal: f64,
        energy: f64,
    }
    let mut usage_props_max = FnvHashMap::<Id<Player>, ResourceUsagePropMax>::default();
    for (player_id, player) in players.iter_mut() {
        if let Some(ru) = resources_usage.get(player_id) {
            let current_metal_stock = player.metal;
            let current_energy_stock = player.energy;

            let metal_needed = ru.metal;
            let energy_needed = ru.energy;

            //TODO Energy count too
            let metal_prop_max: f64 = (current_metal_stock / metal_needed)
                .min(current_energy_stock / energy_needed)
                .min(1.0);
            usage_props_max.insert(
                *player_id,
                ResourceUsagePropMax {
                    metal: metal_prop_max,
                    energy: 1.0,
                },
            );

            player.metal = (player.metal - metal_needed * metal_prop_max).max(0.0);
        }
    }

    //SYNC beause we modify player directly instead of creating another step next
    //Compute build percent for each unit, refund player for overcost
    struct ResourceSurplus {
        metal: f64,
        energy: f64,
    }

    let mut resources_surplus = FnvHashMap::<Id<Player>, ResourceSurplus>::default();
    for BuildPart {
        amount,
        to,
        from,
        player,
        repair,
    } in build_throughputs
    {
        let kbot = kbots.get_mut(&to).unwrap();
        let botdef = bot_defs.get(&kbot.botdef_id).unwrap();
        let metal_available = amount * usage_props_max.get(&player).unwrap().metal;
        let metal_needed = if repair {
            0.0
        } else {
            (1.0 - kbot.con_completed as f64) * botdef.metal_cost as f64
        };
        let mut metal_used = metal_available;
        if metal_needed > metal_available {
            let metal_built =
                metal_available + kbot.con_completed as f64 * botdef.metal_cost as f64;

            kbot.con_completed = (metal_built / botdef.metal_cost as f64) as f32;
        } else {
            let metal_not_used = metal_available - metal_needed;
            metal_used = metal_available - metal_not_used;
            if !repair {
                players.get_mut(&player).unwrap().metal += metal_not_used;
                kbot.con_completed = 1.0;
            }
        }
        let lambda = if repair {
            amount as f32
        } else {
            metal_used as f32
        } / botdef.metal_cost as f32;
        kbot.life = ((kbot.life as f32 + lambda * botdef.max_life as f32).ceil() as i32)
            .min((botdef.max_life as f32 * kbot.con_completed).ceil() as i32);
    }

    frame_profiler.add("01b build compute", start.elapsed());

    //Movement compute

    for (id, mobile) in kbots.iter_mut() {
        if mobile.con_completed >= 1.0 {
            if mobile.speed.magnitude_squared() > 0.001
                || mobile.move_target.is_some()
                || !mobile.grounded
            {
                let botdef = bot_defs.get(&mobile.botdef_id).unwrap();
                let grid_pos = grid_pos(mobile);
                let mut neighbors_id: Vec<Id<KBot>> = grid[grid_pos].clone();
                let to_remove = neighbors_id.iter().position(|e| e == id).unwrap();
                neighbors_id.remove(to_remove);

                let avoidance_force = avoid_neighbors_force(mobile, neighbors_id, &mobiles2) * 0.3;

                let TargetForce {
                    target_force,
                    stop_tracking,
                } = to_target_force(mobile, botdef);

                // arrows.push(Arrow {
                //     position: mobile.position,
                //     color: [target_force.norm(), 0.0, 0.0, 0.0],
                //     end: mobile.position
                //         + Vector3::new(target_force.x * 2.0, target_force.y * 2.0, 0.0),
                // });

                // arrows.push(Arrow {
                //     position: mobile.position,
                //     color: [0.0, avoidance_force.norm(), 0.0, 0.0],
                //     end: mobile.position
                //         + Vector3::new(avoidance_force.x * 2.0, avoidance_force.y * 2.0, 0.0),
                // });

                if stop_tracking {
                    mobile.move_target = None;
                }

                let dir = avoidance_force + target_force;
                let dir_intensity = (avoidance_force.norm() + target_force.norm())
                    .max(0.0)
                    .min(1.0);

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
                let new_dir: Vector2<f32> = new_angle.into();
                mobile.dir = Vector3::new(new_dir.x, new_dir.y, 0.0);

                //TODO drift factor ?
                //drift = 1 (adherence = 0)
                // mobile.speed = mobile.speed + mobile.dir * botdef.accel * dir_intensity;
                //drift = 0 (adherence = 1)

                let speed_scalar = mobile.speed.xy().magnitude();
                let thrust = if speed_scalar > 0.01 {
                    dir.normalize().dot(&(mobile.speed.xy() / speed_scalar))
                } else {
                    1.0
                };

                let accel = if mobile.move_target != None && thrust > 0.0 {
                    botdef.accel * dir_intensity * thrust
                } else {
                    -botdef.break_accel * thrust.abs()
                };

                // arrows.push(Arrow {
                //     position: mobile.position + Vector3::new(0.0, 0.0, 2.0),
                //     color: [0.0, 0.0, accel, 0.0],
                //     end: mobile.position
                //         + Vector3::new(dir.x, dir.y, 0.0) * 4.0
                //         + Vector3::new(0.0, 0.0, 2.0),
                // });

                // arrows.push(Arrow {
                //     position: mobile.position + Vector3::new(0.0, 0.0, 1.0),
                //     color: [0.0, 0.0, accel, 0.0],
                //     end: mobile.position + mobile.dir * accel * 4.0 + Vector3::new(0.0, 0.0, 1.0),
                // });

                mobile.speed = mobile.dir * (accel + mobile.speed.magnitude()).max(0.0);

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
                mobile.position.z = heightmap_phy.z_linear(mobile.position.x, mobile.position.y);
                mobile.grounded = true;
                mobile.up = heightmap_phy.normal(mobile.position.x, mobile.position.y);

                let y = -mobile.dir.cross(&mobile.up);
                let x = y.cross(&mobile.up);
                mobile.dir = x;

                mobile.weapon0_dir = (mobile.weapon0_dir + mobile.dir).normalize();
                //w = v/r
                mobile.wheel0_angle += mobile.speed.norm() / 0.5;
            }
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

fn avoid_neighbors_force(
    me: &KBot,
    neighbors_id: Vec<Id<KBot>>,
    kbots: &FnvHashMap<Id<KBot>, KBot>,
) -> Vector2<f32> {
    // could be speed/ brake
    // let prediction = 1.0;
    let pos = me.position + me.speed;

    let mut avoidance = Vector2::new(0.0, 0.0);
    for other_id in neighbors_id.iter() {
        let other = kbots.get(other_id).unwrap();
        let o_pos = other.position + other.speed;

        let to_other = (o_pos.coords - pos.coords).xy();
        let distance = (to_other.magnitude() - 1.1).max(0.1);
        let inv_distance = 1.0 / distance;
        let to_other_normalized = to_other * inv_distance;
        avoidance += -to_other_normalized * inv_distance;
    }
    avoidance
}

struct TargetForce {
    target_force: Vector2<f32>,
    stop_tracking: bool,
}
fn to_target_force(me: &KBot, botdef: &botdef::BotDef) -> TargetForce {
    if let Some(target) = me.move_target {
        let to_target = (target.coords - (me.position.coords + me.speed)).xy();
        let to_target_distance = to_target.norm();
        let will_to_go_target = if to_target_distance > botdef.radius {
            1.0
        } else {
            1.0 - ((botdef.radius - to_target_distance) / botdef.radius)
        };

        TargetForce {
            target_force: (to_target / to_target_distance) * will_to_go_target,
            stop_tracking: to_target_distance < botdef.radius / 2.0,
        }
    } else {
        TargetForce {
            target_force: Vector2::new(0.0, 0.0),
            stop_tracking: true,
        }
    }
}

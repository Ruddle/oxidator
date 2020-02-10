extern crate nalgebra as na;
use super::heightmap_editor;
use crate::botdef;
use crate::frame::Frame;
use crate::mobile;
use crate::utils;
use na::{Matrix4, Point3, Vector2, Vector3};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use utils::*;

use crate::frame::Player;
use mobile::*;

#[derive(Clone, Copy, Debug)]
pub struct Explosion {
    pub position: Point3<f32>,
    pub born_sec: f32,
    pub death_sec: f32,
    pub size: f32,
    pub seed: f32,
}

pub struct State {
    pub position: Point3<f32>,
    pub dir: Vector3<f32>,

    pub position_smooth: Point3<f32>,
    pub dir_smooth: Vector3<f32>,

    pub mouse_world_pos: Option<Vector3<f32>>,
    pub screen_center_world_pos: Option<Vector3<f32>>,

    pub heightmap_editor: heightmap_editor::State,

    //Data to interpolate
    pub frame_minus_one: Frame,
    pub frame_zero: Frame,
    pub frame_zero_time_received: Instant,

    //Interpolated from curve
    pub kbots: Vec<(KBot, ClientKbot)>,
    pub server_sec: f32,
    //Extrapolated from events
    pub explosions: Vec<Explosion>,
    pub kinematic_projectiles_cache: HashMap<Id<KinematicProjectile>, KinematicProjectile>,
    pub kinematic_projectiles: Vec<Point3<f32>>,

    pub selected: HashSet<IdValue>,
    pub botdef_selected_for_con: Option<Id<botdef::BotDef>>,

    pub start_time: Instant,
    pub last_frame: Instant,

    pub my_player_id: Option<Id<Player>>,

    pub players: HashMap<Id<Player>, Player>,

    pub fps: u64,

    //parameters
    pub unit_icon_distance: f32,
}

impl State {
    pub fn new() -> Self {
        State {
            position: Point3::new(1024.0, 100.0, 50.0),
            dir: Vector3::new(0.0, 0.3, -1.0),
            position_smooth: Point3::new(0.0, 0.0, 30000.0),
            dir_smooth: Vector3::new(0.0, 0.01, -1.0),

            mouse_world_pos: None,
            screen_center_world_pos: None,

            heightmap_editor: heightmap_editor::State::new(),

            frame_minus_one: Frame::new(),
            frame_zero: Frame::new(),
            frame_zero_time_received: Instant::now(),

            kbots: Vec::new(),
            kinematic_projectiles_cache: HashMap::new(),
            kinematic_projectiles: Vec::new(),

            explosions: Vec::new(),
            server_sec: 0.0,

            selected: HashSet::new(),
            botdef_selected_for_con: None,

            players: HashMap::new(),
            my_player_id: None,

            start_time: Instant::now(),
            last_frame: Instant::now(),
            fps: 144,
            unit_icon_distance: 200.0,
        }
    }

    pub fn handle_new_frame(&mut self, frame: Frame) {
        let time_between = self.frame_zero_time_received.elapsed();
        log::trace!("receive: NewFrame after {:?}", time_between);
        self.frame_zero_time_received = Instant::now();
        self.frame_minus_one = std::mem::replace(&mut self.frame_zero, frame);

        let sec = self.frame_zero.number as f32 / 10.0;
        let mut seed = sec * 3.141592;

        for explosion in self.frame_zero.explosions.iter() {
            seed += 1.0;
            self.explosions.push(Explosion {
                position: explosion.position,
                born_sec: sec,
                death_sec: sec + explosion.life_time,
                size: explosion.size,
                seed,
            });
        }

        for proj_b in self.frame_zero.kinematic_projectiles_birth.iter() {
            self.kinematic_projectiles_cache
                .insert(proj_b.id, proj_b.clone());
        }

        for dead in self.frame_zero.kinematic_projectiles_dead.iter() {
            self.kinematic_projectiles_cache.remove(dead);
        }

        self.selected = self
            .selected
            .difference(&self.frame_zero.kbots_dead.iter().map(|e| e.value).collect())
            .copied()
            .collect();

        self.kbots = self
            .frame_zero
            .kbots
            .values()
            .map(|kbot| (kbot.clone(), ClientKbot::new(kbot.position)))
            .collect();
    }

    pub fn interpolate(&mut self, threadpool: &rayon::ThreadPool, view_proj: &Matrix4<f32>) {
        let elapsed = self.frame_zero_time_received.elapsed().as_secs_f64();
        //elapsed normalize between 0 and 1 if frame arrives every 100ms (0.1s)
        let lambda = (elapsed / 0.1) as f32;
        let i0 = lambda;
        let im = 1.0 - lambda;

        self.server_sec =
            (self.frame_zero.number as f32 * i0 + self.frame_minus_one.number as f32 * im) / 10.0;

        log::trace!("server_sec {}", self.server_sec);

        use rayon::prelude::*;
        fn test_screen(
            id: Id<KBot>,
            position: Point3<f32>,
            view_proj: &Matrix4<f32>,
        ) -> Option<(Id<KBot>, Vector2<f32>, f32)> {
            let p = position.to_homogeneous();
            let r = view_proj * p;
            let margin = 1.2;
            let rw_with_margin = r.w * margin;
            //Keeping those of the clipped space in screen (-1 1, -1 1 , 0 1)
            if r.z > 0.0
                && r.x < rw_with_margin
                && r.x > -rw_with_margin
                && r.y < rw_with_margin
                && r.y > -rw_with_margin
            {
                // log::debug!("z {}", r.w);
                // log::debug!("d {}", (position.coords - cam_pos.coords).norm());
                Some((id, Vector2::new(r.x / r.w, r.y / r.w), r.w))
            } else {
                None
            }
        }

        self.explosions = self
            .explosions
            .iter()
            .copied()
            .filter(|e| e.death_sec > self.server_sec)
            .collect();

        let mut kbots = std::mem::replace(&mut self.kbots, Vec::new());

        threadpool.install(|| {
            kbots.par_chunks_mut(1000).for_each(|chunk| {
                for (kbot_0, client_kbot0) in chunk.iter_mut() {
                    let kbot_m_opt = self.frame_minus_one.kbots.get(&kbot_0.id);
                    if let Some(kbot_m) = kbot_m_opt {
                        client_kbot0.position =
                            kbot_0.position * i0 + (im * kbot_m.position).coords;
                    }

                    let screen = test_screen(kbot_0.id, client_kbot0.position, view_proj);
                    match screen {
                        Some((_, screen_pos, distance_to_camera)) => {
                            if let Some(kbot_m) = kbot_m_opt {
                                client_kbot0.dir = kbot_0.dir * i0 + kbot_m.dir * im;
                                client_kbot0.up = kbot_0.up * i0 + kbot_m.up * im;
                                client_kbot0.weapon0_dir =
                                    kbot_0.weapon0_dir * i0 + kbot_m.weapon0_dir * im;
                                client_kbot0.wheel0_angle =
                                    kbot_0.wheel0_angle * i0 + kbot_m.wheel0_angle * im;
                            }
                            client_kbot0.is_in_screen = true;
                            let mat = utils::face_towards_dir(
                                &client_kbot0.position.coords,
                                &(client_kbot0.dir.normalize()),
                                &client_kbot0.up,
                            );
                            client_kbot0.trans = Some(mat);
                            client_kbot0.distance_to_camera = distance_to_camera;
                            client_kbot0.screen_pos = screen_pos;
                        }
                        _ => {
                            client_kbot0.is_in_screen = false;
                        }
                    }
                }
            });
        });

        self.kbots = kbots;
        self.kinematic_projectiles.clear();

        for kproj in self.kinematic_projectiles_cache.values_mut() {
            let pos = kproj.position_at(self.frame_minus_one.number + 1) * im
                + kproj.position_at(self.frame_zero.number + 1).coords * i0;

            self.kinematic_projectiles.push(pos);
        }

        self.players = self.frame_zero.players.clone();
    }

    pub fn my_player(&self) -> Option<&Player> {
        self.my_player_id
            .map(|id| self.players.get(&id))
            .unwrap_or(None)
    }

    pub fn near(&self) -> f32 {
        if self.position_smooth.z > 515.0
            || self.position_smooth.coords.x < -500.0
            || self.position_smooth.coords.y < -500.0
        {
            10.0
        } else {
            0.3
        }
    }
}

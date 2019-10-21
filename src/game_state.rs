extern crate nalgebra as na;
use crate::heightmap_editor;
use crate::mobile;
use crate::utils;
use na::{Point3, Vector3};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use utils::*;

use crate::frame::Player;
use mobile::*;

pub struct State {
    pub position: Point3<f32>,
    pub dir: Vector3<f32>,

    pub position_smooth: Point3<f32>,
    pub dir_smooth: Vector3<f32>,

    pub mouse_world_pos: Option<Vector3<f32>>,
    pub screen_center_world_pos: Option<Vector3<f32>>,

    pub heightmap_editor: heightmap_editor::State,

    pub kbots: HashMap<Id<KBot>, KBot>,

    pub kinematic_projectiles: HashMap<Id<KinematicProjectile>, KinematicProjectile>,
    pub selected: HashSet<IdValue>,

    pub start_time: Instant,
    pub last_frame: Instant,

    pub my_player_id: Option<Id<Player>>,

    pub players: HashMap<Id<Player>, Player>,

    pub fps: u64
}

impl State {
    pub fn new() -> Self {
        State {
            position: Point3::new(1024.0, 100.0, 50.0),

            //            position: Point3::new(1024.0 - 450.0, 1024.0 - 600.0, 1200.0),
            dir: Vector3::new(0.0, 0.3, -1.0),

            position_smooth: Point3::new(0.0, 0.0, 30000.0),
            dir_smooth: Vector3::new(0.0, 0.01, -1.0),

            mouse_world_pos: None,
            screen_center_world_pos: None,

            heightmap_editor: heightmap_editor::State::new(),

            kbots: HashMap::new(),
            kinematic_projectiles: HashMap::new(),
            selected: HashSet::new(),

            players: HashMap::new(),
            my_player_id: None,

            start_time: Instant::now(),
            last_frame: Instant::now(),
              fps: 144
        }
    }

    pub fn my_player(&self) -> Option<&Player> {
        self.my_player_id
            .map(|id| self.players.get(&id))
            .unwrap_or(None)
    }
    pub fn my_player_mut(&mut self) -> Option<&mut Player> {
        match self.my_player_id {
            Some(id) => self.players.get_mut(&id),
            None => None,
        }
    }
}

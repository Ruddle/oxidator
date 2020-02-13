extern crate nalgebra as na;

use crate::heightmap_phy;

use crate::botdef;
use crate::mobile;
use crate::moddef;
use crate::utils;
use fnv::{FnvHashMap, FnvHashSet};
use std::collections::HashMap;
use na::{Point3, Vector3};
use std::time::Duration;

use crate::unit;
use mobile::*;
use serde::{Deserialize, Serialize};
use utils::*;

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct Player {
    pub id: Id<Player>,
    pub kbots: FnvHashSet<Id<KBot>>,
    pub team: u8,
}

impl Player {
    pub fn new() -> Self {
        let id = utils::rand_id();
        Player {
            id,
            kbots: FnvHashSet::default(),
            team: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FrameEventFromPlayer {
    RepairOrder {
        id: Id<Player>,
        selected: FnvHashSet<Id<KBot>>,
        to_repair: Id<KBot>,
    },
    ConOrder {
        id: Id<Player>,
        selected: FnvHashSet<Id<KBot>>,
        mouse_world_pos: Vector3<f32>,
        botdef_id: Id<botdef::BotDef>,
    },
    MoveOrder {
        id: Id<Player>,
        selected: FnvHashSet<Id<KBot>>,
        mouse_world_pos: Vector3<f32>,
    },
    ReplaceFrame(Frame),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfilerMap {
    pub hm: HashMap<String, std::time::Duration>,
}
impl ProfilerMap {
    pub fn new() -> Self {
        ProfilerMap { hm: HashMap::new() }
    }
    pub fn mix(&mut self, s: &str, duration: Duration, last_ratio: u32) {
        match self.hm.get_mut(&s.to_owned()) {
            Some(val) => {
                *val = val
                    .checked_mul(last_ratio)
                    .unwrap()
                    .checked_add(duration)
                    .unwrap()
                    .checked_div(last_ratio + 1)
                    .unwrap();
            }
            None => {
                self.hm.insert(s.to_owned(), duration);
            }
        }
    }

    pub fn add(&mut self, s: &str, duration: Duration) {
        self.hm.insert(s.to_owned(), duration);
    }
    pub fn get(&self, s: &str) -> Option<&Duration> {
        self.hm.get(s)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataToComputeNextFrame {
    pub old_frame: Frame,
    pub events: Vec<FrameEventFromPlayer>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameUpdate {
    pub kbots: Vec<KBot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Frame {
    // relevant to send to client on change
    pub kinematic_projectiles: FnvHashMap<Id<KinematicProjectile>, KinematicProjectile>,
    pub arrows: Vec<Arrow>,
    pub heightmap_phy: Option<heightmap_phy::HeightmapPhy>,
    pub players: FnvHashMap<Id<Player>, Player>,
    pub kbots: FnvHashMap<Id<KBot>, KBot>,
    pub moddef: moddef::ModDef,
    // relevant to send to client once
    pub bot_defs: FnvHashMap<Id<botdef::BotDef>, botdef::BotDef>,
    // relevant to send to client always
    pub number: i32,
    pub explosions: Vec<ExplosionEvent>,
    pub kbots_dead: FnvHashSet<Id<KBot>>,
    pub kinematic_projectiles_dead: Vec<Id<KinematicProjectile>>,
    pub kinematic_projectiles_birth: Vec<KinematicProjectile>,
    pub frame_profiler: ProfilerMap,
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            number: 0,
            players: FnvHashMap::default(),
            moddef: moddef::ModDef::new(),
            kbots: FnvHashMap::default(),
            kinematic_projectiles: FnvHashMap::default(),
            arrows: Vec::new(),
            explosions: Vec::new(),
            heightmap_phy: None,
            frame_profiler: ProfilerMap::new(),
            kbots_dead: FnvHashSet::default(),
            kinematic_projectiles_dead: Vec::new(),
            kinematic_projectiles_birth: Vec::new(),
            bot_defs: FnvHashMap::default(),
        }
    }
}

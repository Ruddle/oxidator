extern crate nalgebra as na;
use crate::heightmap_editor;
use crate::input_state;
use crate::mobile;
use crate::utils;
use na::{Point3, Vector3};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use utils::*;

use mobile::*;

#[derive(Clone, TypeName, Debug)]
pub struct Player {
    pub id: Id<Player>,
    pub mobiles: HashSet<Id<KBot>>,
    pub team: u8,
}

impl Player {
    pub fn new() -> Self {
        let id = utils::rand_id();
        Player {
            id,
            mobiles: HashSet::new(),
            team: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum FrameEvent {
    PlayerInput {
        id: Id<Player>,
        input_state: input_state::InputState,
        selected: HashSet<IdValue>,
        mouse_world_pos: Option<Vector3<f32>>,
    },
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub number: i32,
    pub players: HashMap<Id<Player>, Player>,
    pub kbots: HashMap<Id<KBot>, KBot>,
    pub kinematic_projectiles: HashMap<Id<KinematicProjectile>, KinematicProjectile>,
    pub events: Vec<FrameEvent>,
    pub complete: bool,
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            number: 0,
            players: HashMap::new(),
            kbots: HashMap::new(),
            kinematic_projectiles: HashMap::new(),
            events: Vec::new(),
            complete: true,
        }
    }
}

use crate::utils;
use na::{Matrix4, Point3, Vector3};
use serde::{Deserialize, Serialize};
use typename::TypeName;
use utils::Id;

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct KBot {
    pub position: Point3<f32>,
    pub trans: Option<Matrix4<f32>>,
    pub speed: Vector3<f32>,
    pub dir: Vector3<f32>,
    pub target: Option<Point3<f32>>,
    pub id: Id<KBot>,

    pub radius: f32,
    pub life: i32,
    pub max_life: i32,
    pub team: i32,

    pub grounded: bool,

    pub frame_last_shot: i32,
    pub reload_frame_count: i32,
}

impl KBot {
    pub fn new(position: Point3<f32>) -> Self {
        KBot {
            position,
            speed: Vector3::new(0.0, 0.0, 0.0),
            trans: None,
            team: 0,
            dir: Vector3::new(1.0, 0.0, 0.0),
            target: None,
            id: utils::rand_id(),
            radius: 0.5,
            frame_last_shot: 0,
            reload_frame_count: 3,
            life: 100,
            max_life: 100,
            grounded: false,
        }
    }
}

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct KinematicProjectile {
    pub id: Id<KinematicProjectile>,
    pub positions: Vec<Point3<f32>>,
    pub radius: f32,
}

impl KinematicProjectile {
    pub fn new(positions: Vec<Point3<f32>>) -> Self {
        KinematicProjectile {
            id: utils::rand_id(),
            positions,
            radius: 0.25,
        }
    }

    pub fn position(&self) -> Point3<f32> {
        self.positions.first().unwrap().clone()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Arrow {
    pub position: Point3<f32>,
    pub end: Point3<f32>,
    pub color: [f32; 4],
}

impl Arrow {
    pub fn new(position: Point3<f32>, end: Point3<f32>, color: [f32; 4]) -> Self {
        Arrow {
            position,
            color,
            end,
        }
    }
}

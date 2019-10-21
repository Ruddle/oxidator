use crate::utils;
use na::{Point3, Vector3};
use typename::TypeName;
use utils::Id;

#[derive(Clone, TypeName, Debug)]
pub struct KBot {
    pub position: Point3<f32>,
    pub speed: Vector3<f32>,
    pub dir: Vector3<f32>,
    pub target: Option<Point3<f32>>,
    pub id: Id<KBot>,

    pub frame_last_shot: i32,
    pub reload_frame_count: i32,
}

impl KBot {
    pub fn new(position: Point3<f32>) -> Self {
        KBot {
            position,
            speed: Vector3::new(0.0, 0.0, 0.0),
            dir: Vector3::new(1.0, 0.0, 0.0),
            target: None,
            id: utils::rand_id(),
            frame_last_shot: 0,
            reload_frame_count: 10,
        }
    }
}

#[derive(Clone, TypeName, Debug)]
pub struct KinematicProjectile {
    pub id: Id<KinematicProjectile>,
    pub positions: Vec<Point3<f32>>,
}

impl KinematicProjectile {
    pub fn new(positions: Vec<Point3<f32>>) -> Self {
        KinematicProjectile {
            id: utils::rand_id(),
            positions,
        }
    }
}

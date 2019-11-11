use crate::unit;
use crate::utils;
use na::{Matrix4, Point3, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use typename::TypeName;
use utils::Id;

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExplosionEvent {
    pub position: Point3<f32>,
    pub size: f32,
    pub life_time: f32,
}

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct KBot {
    pub position: Point3<f32>,
    pub trans: Option<Matrix4<f32>>,
    pub is_in_screen: bool,
    pub distance_to_camera: f32,
    pub screen_pos: Vector2<f32>,
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
    pub part_tree: unit::PartTree,
}

impl KBot {
    pub fn new(position: Point3<f32>) -> Self {
        KBot {
            position,
            speed: Vector3::new(0.0, 0.0, 0.0),
            trans: None,
            is_in_screen: false,
            distance_to_camera: 0.0,
            screen_pos: Vector2::new(0.0, 0.0),
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
            part_tree: unit::PartTree {
                id: utils::rand_id(),
                dmodel: None,
                joint: unit::Joint::Fix,
                children: vec![unit::PartTree {
                    id: utils::rand_id(),
                    dmodel: Some(unit::DisplayModel {
                        //Z is Y ?
                        position: Point3::new(0.0, 0.0, 0.0),
                        dir: Vector3::new(1.0, 0.0, 0.0),
                        model_path: Path::new("./src/asset/cube.obj").to_owned(),
                    }),
                    joint: unit::Joint::Fix,
                    children: vec![],
                }],
            },
        }
    }
}

// unit::PartTree {
//     id: utils::rand_id(),
//     dmodel: Some(unit::DisplayModel {
//         //Z is Y ?
//         position: Point3::new(0.5, 0.0, 0.0),
//         dir: Vector3::new(1.0, 0.0, 0.0),
//         model_path: Path::new("./src/asset/small_sphere.obj").to_owned(),
//     }),
//     joint: unit::Joint::Fix,
//     children: Vec::new(),
// }

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct KinematicProjectile {
    pub id: Id<KinematicProjectile>,
    pub birth_frame: i32,
    pub death_frame: i32,
    pub position_at_birth: Point3<f32>,
    pub speed_per_frame_at_birth: Vector3<f32>,
    pub accel_per_frame: Vector3<f32>,
    pub radius: f32,

    pub position_cache: Vec<Point3<f32>>,
    pub speed_cache: Vec<Vector3<f32>>,
}

impl KinematicProjectile {
    pub fn speed_at(&mut self, frame_number: i32) -> Vector3<f32> {
        //End recursion
        if frame_number == self.birth_frame {
            self.speed_per_frame_at_birth
        }
        //Check cache
        else if self.speed_cache.len() as i32 > frame_number - self.birth_frame {
            self.speed_cache[(frame_number - self.birth_frame) as usize]
        }
        //Compute
        else {
            let new_speed = self.speed_at(frame_number - 1) + self.accel_per_frame;
            self.speed_cache.push(new_speed);
            *self.speed_cache.last().unwrap()
        }
    }
    pub fn position_at(&mut self, frame_number: i32) -> Point3<f32> {
        //End recursion
        if frame_number == self.birth_frame {
            self.position_at_birth
        }
        //Check cache
        else if self.position_cache.len() as i32 > frame_number - self.birth_frame {
            self.position_cache[(frame_number - self.birth_frame) as usize]
        }
        //Compute
        else {
            let new_pos = self.position_at(frame_number - 1) + self.speed_at(frame_number);
            self.position_cache.push(new_pos);
            *self.position_cache.last().unwrap()
        }
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

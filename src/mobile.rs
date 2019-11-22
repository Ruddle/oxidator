use crate::botdef;
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Angle {
    pub rad: f32,
}
impl Angle {
    pub fn from(x: f32, y: f32) -> Self {
        Angle {
            rad: f32::atan2(y, x),
        }
        .modulo()
    }

    pub fn new(rad: f32) -> Self {
        Angle { rad }.modulo()
    }

    pub fn clamp_around(self, other: Angle, cone_angle: Angle) -> Self {
        let max = other + cone_angle;
        let min = other - cone_angle;
        let diff = (other - self);
        let diff = diff.rad.max(-cone_angle.rad).min(cone_angle.rad);
        self + Angle::new(diff)
    }

    pub fn modulo(&self) -> Self {
        Angle {
            rad: (self.rad + std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI)
                - std::f32::consts::PI,
        }
    }
}

impl std::ops::Add<Angle> for Angle {
    type Output = Angle;

    fn add(self, rhs: Angle) -> Angle {
        Angle {
            rad: (self.rad + rhs.rad),
        }
        .modulo()
    }
}

impl std::ops::Sub<Angle> for Angle {
    type Output = Angle;

    fn sub(self, rhs: Angle) -> Angle {
        Angle {
            rad: (self.rad - rhs.rad),
        }
        .modulo()
    }
}

impl std::ops::Neg for Angle {
    type Output = Angle;

    fn neg(self) -> Angle {
        Angle {
            rad: (self.rad + std::f32::consts::PI),
        }
        .modulo()
    }
}

impl From<Vector2<f32>> for Angle {
    fn from(dir: Vector2<f32>) -> Self {
        Self::from(dir.x, dir.y)
    }
}

impl Into<Vector2<f32>> for Angle {
    fn into(self) -> Vector2<f32> {
        Vector2::new(f32::cos(self.rad), f32::sin(self.rad))
    }
}

impl From<(f32, f32)> for Angle {
    fn from(dir: (f32, f32)) -> Self {
        Self::from(dir.0, dir.1)
    }
}

impl From<f32> for Angle {
    fn from(rad: f32) -> Self {
        Self::new(rad)
    }
}

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct KBot {
    pub id: Id<KBot>,
    pub position: Point3<f32>,
    pub speed: Vector3<f32>,
    pub dir: Vector3<f32>,
    pub angle: Angle,
    pub angular_velocity: f32,
    pub up: Vector3<f32>,
    pub target: Option<Point3<f32>>,
    pub life: i32,
    pub team: i32,
    pub grounded: bool,
    pub frame_last_shot: i32,
    pub weapon0_dir: Vector3<f32>,
    pub wheel0_angle: f32,
    pub reload_frame_count: i32,
    pub botdef_id: Id<botdef::BotDef>,
}

impl KBot {
    pub fn new(position: Point3<f32>, botdef: &botdef::BotDef) -> Self {
        KBot {
            position,
            speed: Vector3::new(0.0, 0.0, 0.0),
            team: 0,
            dir: Vector3::new(1.0, 0.0, 0.0),
            angle: Angle::new(0.0),
            up: Vector3::new(0.0, 0.0, 1.0),
            target: None,
            id: utils::rand_id(),
            frame_last_shot: 0,
            reload_frame_count: 3,
            weapon0_dir: Vector3::new(1.0, 0.0, 0.0),
            wheel0_angle: 0.0,
            life: botdef.max_life,
            grounded: false,
            botdef_id: botdef.id,
            angular_velocity: 0.0,
        }
    }
}

pub struct ClientKbot {
    pub position: Point3<f32>,
    pub dir: Vector3<f32>,
    pub up: Vector3<f32>,

    pub weapon0_dir: Vector3<f32>,
    pub wheel0_angle: f32,

    pub trans: Option<Matrix4<f32>>,
    pub is_in_screen: bool,
    pub distance_to_camera: f32,
    pub screen_pos: Vector2<f32>,
}

impl ClientKbot {
    pub fn new(position: Point3<f32>) -> Self {
        ClientKbot {
            position,
            dir: Vector3::new(1.0, 0.0, 0.0),
            up: Vector3::new(0.0, 0.0, 1.0),
            weapon0_dir: Vector3::new(1.0, 0.0, 0.0),
            wheel0_angle: 0.0,
            trans: None,
            is_in_screen: false,
            distance_to_camera: 0.0,
            screen_pos: Vector2::new(0.0, 0.0),
        }
    }
}

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

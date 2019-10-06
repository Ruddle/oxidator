extern crate nalgebra as na;
use na::{Matrix4, Point3, Rotation3, Vector3};
use std::collections::HashSet;
use std::time::Instant;

pub struct State {
    pub position: Point3<f32>,
    pub dir: Vector3<f32>,
    pub key_pressed: HashSet<winit::event::VirtualKeyCode>,
    pub fps: u64,
    pub last_frame: Instant,
}

impl State {
    pub fn new() -> Self {
        State {
            position: Point3::new(0.0, 0.0, 0.0),
            dir: Vector3::new(1.0, 0.0, 0.0),
            key_pressed: HashSet::new(),
            fps: 144,
            last_frame: Instant::now(),
        }
    }
}

extern crate nalgebra as na;
use crate::heightmap_editor;
use na::{Matrix4, Point3, Rotation3, Vector3};
use std::time::Instant;

pub struct State {
    pub position: Point3<f32>,
    pub dir: Vector3<f32>,

    pub position_smooth: Point3<f32>,
    pub dir_smooth: Vector3<f32>,

    pub mouse_world_pos: Vector3<f32>,

    pub heightmap_editor: heightmap_editor::State,

    pub start_time: Instant,
    pub last_frame: Instant,
}

impl State {
    pub fn new() -> Self {
        State {
            position: Point3::new(0.0, 0.0, 30.0),
            dir: Vector3::new(0.0, 0.3, -1.0),

            position_smooth: Point3::new(0.0, 0.0, 30000.0),
            dir_smooth: Vector3::new(0.0, 0.01, -1.0),

            mouse_world_pos: Vector3::new(0.0, 0.0, 0.0),

            heightmap_editor: heightmap_editor::State::new(),

            start_time: Instant::now(),
            last_frame: Instant::now(),
        }
    }
}

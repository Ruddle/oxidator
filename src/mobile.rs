use na::{Point3, Vector3};

pub struct Mobile {
    pub position: Point3<f32>,
    pub speed: Vector3<f32>,
    pub yaw: f32,
}

impl Mobile {
    pub fn new(position: Point3<f32>) -> Self {
        Mobile {
            position,
            speed: Vector3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
        }
    }
}

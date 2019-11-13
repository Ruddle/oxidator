use na::Vector3;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HeightmapPhy {
    pub texels: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

trait HeightMapPhyUsize {
    fn z(&self, x: usize, y: usize) -> f32;
}

impl HeightMapPhyUsize for HeightmapPhy {
    fn z(&self, x: usize, y: usize) -> f32 {
        self.texels[x + y * self.width]
    }
}

impl HeightmapPhy {
    pub fn new(width: usize, height: usize) -> Self {
        let mut texels = Vec::with_capacity((width * height) as usize);
        for j in 0..height {
            for i in 0..width {
                texels.push(50.0);
            }
        }
        HeightmapPhy {
            texels,
            width,
            height,
        }
    }

    ///unsafe nearest interpolation
    #[inline]
    pub fn z(&self, x: f32, y: f32) -> f32 {
        let i = x as usize + (y as usize) * self.width as usize;
        self.texels[i]
    }

    ///safe nearest interpolation
    #[inline]
    pub fn safe_z(&self, x: f32, y: f32) -> f32 {
        let x = x.max(0.0).min(self.width as f32 - 1.0);
        let y = y.max(0.0).min(self.height as f32 - 1.0);
        self.z(x, y)
    }

    ///safe linear interpolation
    pub fn z_linear(&self, x: f32, y: f32) -> f32 {
        let x = x.max(0.0).min(self.width as f32 - 2.0);
        let y = y.max(0.0).min(self.height as f32 - 2.0);
        let imin = x.trunc() as usize;
        let imax = imin + 1;
        let jmin = y.trunc() as usize;
        let jmax = self.width as usize * (jmin + 1);
        let jmin = self.width as usize * jmin;

        let a = self.texels[imin + jmin];
        let b = self.texels[imax + jmin];
        let c = self.texels[imax + jmax];
        let d = self.texels[imin + jmax];

        let z = a * (1.0 - x.fract()) * (1.0 - y.fract())
            + b * (x.fract()) * (1.0 - y.fract())
            + c * (x.fract()) * (y.fract())
            + d * (1.0 - x.fract()) * (y.fract());

        z
    }

    ///safe normal interpolation
    pub fn normal(&self, x: f32, y: f32) -> Vector3<f32> {
        let x = x.max(1.0).min(self.width as f32 - 2.0);
        let y = y.max(1.0).min(self.height as f32 - 2.0);

        let r = self.z_linear(x + 1.0, y);
        let l = self.z_linear(x - 1.0, y);
        let u = self.z_linear(x, y - 1.0);
        let d = self.z_linear(x, y + 1.0);
        Vector3::new(-(r - l), d - u, 2.0).normalize()
    }
}

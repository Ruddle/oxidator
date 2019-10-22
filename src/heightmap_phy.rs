use crate::heightmap;

#[derive(Clone, Debug)]
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
        let texels = heightmap::create_texels(width as u32, height as u32, 0.0);
        HeightmapPhy {
            texels,
            width,
            height,
        }
    }

    pub fn z(&self, x: f32, y: f32) -> f32 {
        let i = x as usize + (y as usize) * self.width as usize;
        self.texels[i]
    }

    pub fn z_linear(&self, x: f32, y: f32) -> f32 {
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
}

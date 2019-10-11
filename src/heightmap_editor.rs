use imgui::*;
use na::Vector3;
use std::collections::HashSet;

use crate::heightmap_gpu;
use noise::NoiseFn;
use wgpu::{CommandEncoder, Device};

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Raise,
    Smooth,
    Noise,
}

pub struct State {
    pub pen_radius: u32,
    pub pen_strength: f32,
    pub mode: Mode,
    noise: noise::Perlin,
    noise_scale: f64,
}

impl State {
    pub fn new() -> Self {
        State {
            pen_radius: 50,
            pen_strength: 1.0,
            mode: Mode::Raise,
            noise: noise::Perlin::new(),
            noise_scale: 10.0,
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        let pen_radius = &mut self.pen_radius;
        let pen_strength = &mut self.pen_strength;
        let mode = &mut self.mode;
        let noise_scale = &mut self.noise_scale;
        let edit_height_window = imgui::Window::new(im_str!("Heightmap editor"));
        edit_height_window
            .size([400.0, 200.0], imgui::Condition::FirstUseEver)
            .position([3.0, 206.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.radio_button(im_str!("Raise/Lower"), mode, Mode::Raise);
                ui.radio_button(im_str!("Smooth/Sharpen"), mode, Mode::Smooth);
                ui.radio_button(im_str!("Noise"), mode, Mode::Noise);

                if mode == &mut Mode::Noise {
                    imgui::Slider::new(im_str!("noise scale"), 0.0_f64..=100.0)
                        .build(&ui, noise_scale);
                }

                imgui::Slider::new(im_str!("pen radius"), 1..=1000).build(&ui, pen_radius);
                imgui::Slider::new(im_str!("pen strength"), 0.0..=10.0).build(&ui, pen_strength);
            });
    }

    pub fn handle_user_input(
        &self,
        mouse_pressed: &HashSet<winit::event::MouseButton>,
        mouse_world_pos: &Vector3<f32>,
        heightmap_gpu: &mut heightmap_gpu::HeightmapGpu,
        device: &Device,
        encoder: &mut CommandEncoder,
    ) {
        {
            let pen_strength = self.pen_strength
                * if mouse_pressed.contains(&winit::event::MouseButton::Left) {
                    1.0
                } else if mouse_pressed.contains(&winit::event::MouseButton::Right) {
                    -1.0
                } else {
                    0.0
                };

            if pen_strength != 0.0 {
                let (x, y) = (mouse_world_pos.x, mouse_world_pos.y);

                let middle_i = x.floor() as i32;
                let middle_j = y.floor() as i32;

                let pen_size = self.pen_radius as i32;
                let pen_size2 = pen_size * pen_size;

                let min_i = (middle_i - pen_size).max(0);
                let min_j = (middle_j - pen_size).max(0);

                let max_i = (middle_i + pen_size).min(heightmap_gpu.width as i32 - 1);
                let max_j = (middle_j + pen_size).min(heightmap_gpu.height as i32 - 1);

                let size_i = max_i - min_i + 1;
                let size_j = max_j - min_j + 1;

                if size_i > 0 && size_j > 0 {
                    let mut new_texels = Vec::new();
                    let pixels: Vec<(i32, i32)> = (min_j..=max_j)
                        .flat_map(|j| (min_i..=max_i).map(move |i| (i, j)))
                        .collect();

                    match self.mode {
                        Mode::Raise => {
                            for (i, j) in pixels {
                                let distance2 =
                                    (i32::pow(i - middle_i, 2) + i32::pow(j - middle_j, 2)) as f32;

                                let power = pen_strength * (pen_size2 as f32 - distance2).max(0.0)
                                    / (pen_size2 as f32);

                                let z = heightmap_gpu.texels
                                    [(i + j * heightmap_gpu.width as i32) as usize]
                                    + power;
                                new_texels.push(z);
                            }
                        }
                        Mode::Smooth => {
                            let mut average = 0.0;
                            for (i, j) in &pixels {
                                let z = heightmap_gpu.texels
                                    [(i + j * heightmap_gpu.width as i32) as usize];
                                average += z;
                            }
                            average /= (size_i * size_j) as f32;
                            for (i, j) in pixels {
                                let distance2 =
                                    (i32::pow(i - middle_i, 2) + i32::pow(j - middle_j, 2)) as f32;

                                let power = (pen_strength
                                    * (pen_size2 as f32 - distance2).max(0.0)
                                    / (pen_size2 as f32))
                                    / 50.0;

                                let z = heightmap_gpu.texels
                                    [(i + j * heightmap_gpu.width as i32) as usize]
                                    * (1.0 - power)
                                    + average * power;
                                new_texels.push(z);
                            }
                        }
                        Mode::Noise => {
                            for (i, j) in pixels {
                                let distance2 =
                                    (i32::pow(i - middle_i, 2) + i32::pow(j - middle_j, 2)) as f32;

                                let power = pen_strength * (pen_size2 as f32 - distance2).max(0.0)
                                    / (pen_size2 as f32);

                                let z = heightmap_gpu.texels
                                    [(i + j * heightmap_gpu.width as i32) as usize]
                                    + power
                                        * self.noise.get([
                                            0.0005 * self.noise_scale * i as f64,
                                            0.0005 * self.noise_scale * j as f64,
                                        ]) as f32;

                                new_texels.push(z);
                            }
                        }
                    }

                    heightmap_gpu.update(
                        min_i as u32,
                        min_j as u32,
                        size_i as u32,
                        size_j as u32,
                        new_texels,
                        device,
                        encoder,
                    );
                }
            }
        }
    }
}

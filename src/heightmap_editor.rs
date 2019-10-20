use imgui::*;
use na::Vector3;
use std::collections::HashSet;

use crate::heightmap_gpu;
use noise::{NoiseFn, Seedable};

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Raise,
    Flatten,
    Median,
    Noise,
    Blur,
}

pub struct State {
    pub pen_radius: u32,
    pub pen_strength: f32,
    pub mode: Mode,
    noise: noise::Perlin,
    noise_freq: f64,
    min_z: f32,
    max_z: f32,
}

impl State {
    pub fn new() -> Self {
        State {
            pen_radius: 30,
            pen_strength: 2.0,
            mode: Mode::Raise,
            noise: noise::Perlin::new().set_seed(0),
            noise_freq: 10.0,
            min_z: 0.0,
            max_z: heightmap_gpu::MAX_Z,
        }
    }

    pub fn draw_ui(&mut self, ui: &Ui, heightmap_gpu: &mut heightmap_gpu::HeightmapGpu) {
        let pen_radius = &mut self.pen_radius;
        let pen_strength = &mut self.pen_strength;
        let mode = &mut self.mode;
        let noise_freq = &mut self.noise_freq;
        let noise_seed: &mut i32 = &mut (self.noise.seed() as i32);
        let mut update_noise = false;
        let mut save = false;
        let min_z = &mut self.min_z;
        let max_z = &mut self.max_z;
        let edit_height_window = imgui::Window::new(im_str!("Heightmap editor"));
        edit_height_window
            .size([400.0, 300.0], imgui::Condition::FirstUseEver)
            .position([3.0, 206.0], imgui::Condition::FirstUseEver)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.radio_button(im_str!("Raise/Lower"), mode, Mode::Raise);
                ui.radio_button(im_str!("Flatten/Unflatten"), mode, Mode::Flatten);
                ui.radio_button(im_str!("Median"), mode, Mode::Median);
                ui.radio_button(im_str!("Blur"), mode, Mode::Blur);
                ui.radio_button(im_str!("Noise"), mode, Mode::Noise);

                if mode == &mut Mode::Noise {
                    imgui::Slider::new(im_str!("noise frequency"), 0.0_f64..=200.0)
                        .power(3.0)
                        .build(&ui, noise_freq);

                    update_noise = ui
                        .drag_int(im_str!("noise seed"), noise_seed)
                        .min(0)
                        .build();
                    ui.separator();
                } else {
                    ui.separator();
                }

                imgui::Slider::new(im_str!("pen radius"), 1..=1000).build(&ui, pen_radius);
                imgui::Slider::new(im_str!("pen strength"), 0.0..=10.0).build(&ui, pen_strength);
                ui.separator();

                imgui::Slider::new(im_str!("min height"), 0.0..=heightmap_gpu::MAX_Z)
                    .build(&ui, min_z);
                imgui::Slider::new(im_str!("max height"), 0.0..=heightmap_gpu::MAX_Z)
                    .build(&ui, max_z);

                if ui.small_button(im_str!("Save")) {
                    Self::save(heightmap_gpu);
                }

                if ui.small_button(im_str!("Clear")) {
                    for i in 0..heightmap_gpu.width * heightmap_gpu.height {
                        heightmap_gpu.texels[i as usize] = 0.0;
                    }
                    heightmap_gpu.update_rect(
                        0 as u32,
                        0 as u32,
                        heightmap_gpu.width as u32,
                        heightmap_gpu.height as u32,
                    );
                }

                if ui.small_button(im_str!("Load")) {
                    Self::load(heightmap_gpu);
                }
            });

        self.max_z = max_z.max(*min_z);
        if update_noise {
            self.noise = self.noise.set_seed(*noise_seed as u32);
        }
    }

    pub fn handle_user_input(
        &self,
        mouse_pressed: &HashSet<winit::event::MouseButton>,
        mouse_world_pos: &Vector3<f32>,
        heightmap_gpu: &mut heightmap_gpu::HeightmapGpu,
    ) {
        log::trace!("heightmap_editor handle_user_input");
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
                    //let start = std::time::Instant::now();

                    let mut pixels = Vec::with_capacity((size_i * size_j) as usize);
                    for j in min_j..=max_j {
                        for i in min_i..=max_i {
                            let falloff = 1.0
                                - (i32::pow(i - middle_i, 2) + i32::pow(j - middle_j, 2)) as f32
                                    / pen_size2 as f32;

                            pixels.push((
                                i,
                                j,
                                (i + j * heightmap_gpu.width as i32) as usize,
                                falloff.max(0.0),
                            ));
                        }
                    }

                    match self.mode {
                        Mode::Raise => {
                            for (_, _, index, falloff) in pixels {
                                let power = pen_strength * falloff;
                                heightmap_gpu.texels[index] = (heightmap_gpu.texels[index] + power)
                                    .min(self.max_z)
                                    .max(self.min_z);
                            }
                        }
                        Mode::Flatten => {
                            let mut average = 0.0;
                            for (_, _, index, _) in &pixels {
                                let z = heightmap_gpu.texels[*index];
                                average += z;
                            }
                            average /= (size_i * size_j) as f32;
                            for (_, _, index, falloff) in pixels {
                                let power = (pen_strength * falloff) / 50.0;
                                let z =
                                    heightmap_gpu.texels[index] * (1.0 - power) + average * power;
                                heightmap_gpu.texels[index] = z.min(self.max_z).max(self.min_z);
                            }
                        }
                        Mode::Noise => {
                            for (i, j, index, falloff) in pixels {
                                let power = pen_strength
                                    * falloff
                                    * self.noise.get([
                                        (0.001 * self.noise_freq) * i as f64,
                                        (0.001 * self.noise_freq) * j as f64,
                                    ]) as f32;

                                heightmap_gpu.texels[index] = (heightmap_gpu.texels[index] + power)
                                    .min(self.max_z)
                                    .max(self.min_z);
                            }
                        }
                        Mode::Median => {
                            let mut new_pix = Vec::new();
                            for (i, j, index, _) in pixels {
                                let power = pen_strength / 10.0;

                                let kernel = 4;
                                let mut acc = Vec::new();

                                for ti in (-kernel + i).max(0)
                                    ..=(kernel + i).min(heightmap_gpu.width as i32 - 1)
                                {
                                    for tj in (-kernel + j).max(0)
                                        ..=(kernel + j).min(heightmap_gpu.height as i32 - 1)
                                    {
                                        let tindex =
                                            (ti + tj * heightmap_gpu.width as i32) as usize;
                                        acc.push(
                                            (heightmap_gpu.texels[tindex] * 1000.0 * 1000.0).floor()
                                                as i128,
                                        );
                                    }
                                }
                                acc.sort();
                                new_pix.push((
                                    index,
                                    heightmap_gpu.texels[index] * (1.0 - power)
                                        + power * (acc[acc.len() / 2] as f64 / 1000000.0) as f32,
                                ));
                            }
                            for (index, z) in new_pix {
                                heightmap_gpu.texels[index] = z.min(self.max_z).max(self.min_z);
                            }
                        }
                        Mode::Blur => {
                            let mut new_pix = Vec::new();
                            for (i, j, index, falloff) in pixels {
                                let power = pen_strength * falloff / 10.0;

                                let kernel = 1;
                                let mut acc = 0.0;
                                let mut tap = 0;

                                for ti in (-kernel + i).max(0)
                                    ..=(kernel + i).min(heightmap_gpu.width as i32 - 1)
                                {
                                    for tj in (-kernel + j).max(0)
                                        ..=(kernel + j).min(heightmap_gpu.height as i32 - 1)
                                    {
                                        tap += 1;
                                        let tindex =
                                            (ti + tj * heightmap_gpu.width as i32) as usize;
                                        acc += heightmap_gpu.texels[tindex];
                                    }
                                }
                                let z = heightmap_gpu.texels
                                    [(i + j * heightmap_gpu.width as i32) as usize]
                                    * (1.0 - power)
                                    + power * (acc / tap as f32);
                                new_pix.push((index, z));
                            }
                            for (index, z) in new_pix {
                                heightmap_gpu.texels[index] = z.min(self.max_z).max(self.min_z);
                            }
                        }
                    }

                    heightmap_gpu.update_rect(
                        min_i as u32,
                        min_j as u32,
                        size_i as u32,
                        size_j as u32,
                    );
                    //                    println!("handle hei took {}", start.elapsed().as_micros());
                }
            }
        }
    }

    pub fn save(heightmap_gpu: &heightmap_gpu::HeightmapGpu) {
        //         For reading and opening files
        use std::fs::File;
        use std::io::BufWriter;
        use std::path::Path;

        let path = Path::new(r"heightmap.png");
        let file = File::create(path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, heightmap_gpu.width, heightmap_gpu.height); // Width is 2 pixels and height is 1.
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();

        let data: Vec<u8> = heightmap_gpu
            .texels
            .iter()
            .map(|e| ((e / 511.0).min(1.0).max(0.0) * 65535.0) as u16)
            .flat_map(|e| vec![(e >> 8) as u8, e as u8])
            .collect();
        //        let data = &data[..] ;//[255, 0, 0, 255, 0, 0, 0, 255]; // An array containing a RGBA sequence. First pixel is red and second pixel is black.
        writer.write_image_data(&data).unwrap(); // Save
    }

    pub fn load(heightmap_gpu: &mut heightmap_gpu::HeightmapGpu) {
        use byteorder::{BigEndian, ReadBytesExt};
        use std::fs::File;

        use std::io::Cursor;

        // The decoder is a build for reader and can be used to set various decoding options
        // via `Transformations`. The default output transformation is `Transformations::EXPAND
        // | Transformations::STRIP_ALPHA`.
        let mut decoder = png::Decoder::new(File::open(r"heightmap.png").unwrap());
        decoder.set_transformations(png::Transformations::IDENTITY);
        let (info, mut reader) = decoder.read_info().unwrap();

        // Display image metadata.
        println!("info: {:?}", info.width);
        println!("height: {:?}", info.height);
        println!("bit depth: {:?}", info.bit_depth);
        println!("buffer size: {:?}", info.buffer_size());

        // Allocate the output buffer.
        let mut buf = vec![0; info.buffer_size()];
        // Read the next frame. Currently this function should only called once.
        // The default options
        reader.next_frame(&mut buf).unwrap();

        // Transform buffer into 16 bits slice.
        let mut buffer_u16 = vec![0; (info.width * info.height) as usize];
        let mut buffer_cursor = Cursor::new(buf);
        buffer_cursor
            .read_u16_into::<BigEndian>(&mut buffer_u16)
            .unwrap();

        for i in 0..heightmap_gpu.width * heightmap_gpu.height {
            heightmap_gpu.texels[i as usize] = buffer_u16[i as usize] as f32 / (65535.0 / 511.0);
        }
        heightmap_gpu.update_rect(
            0 as u32,
            0 as u32,
            heightmap_gpu.width as u32,
            heightmap_gpu.height as u32,
        );
    }
}

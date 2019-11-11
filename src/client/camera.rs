extern crate nalgebra as na;
use super::client::*;
use na::{Matrix4, Point3, Vector3};

const FOVY: f32 = 3.14 / 4.0;
const NEAR: f32 = 1.0;
const FAR: f32 = 8000.0;

pub fn create_view(pos: &Point3<f32>, dir: &Vector3<f32>) -> Matrix4<f32> {
    Matrix4::look_at_rh(pos, &(pos + dir), &Vector3::new(0.0, 0.0, 1.0))
}

pub fn create_normal(pos: &Point3<f32>, dir: &Vector3<f32>) -> Matrix4<f32> {
    Matrix4::look_at_rh(pos, &(pos + dir), &Vector3::new(0.0, 0.0, 1.0))
        .try_inverse()
        .unwrap()
        .transpose()
}

pub fn create_proj(aspect_ratio: f32, near: f32) -> Matrix4<f32> {
    let mx_projection = Matrix4::new_perspective(aspect_ratio, FOVY, near, FAR);
    let mx_correction: Matrix4<f32> = Matrix4::new(
        1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 1.0,
    );
    mx_correction * mx_projection
}

pub fn create_view_proj(aspect_ratio: f32, near: f32, pos: &Point3<f32>, dir: &Vector3<f32>) -> Matrix4<f32> {
    let mx_view = create_view(pos, dir);
    let mx_proj = create_proj(aspect_ratio, near);
    mx_proj * mx_view
}

pub fn update_camera_uniform(
    screen_res: (u32, u32),
     near: f32,
    pos: &Point3<f32>,
    dir: &Vector3<f32>,
    uniform_buf: &wgpu::Buffer,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
) {
    //ViewProj
    let mx_total = create_view_proj(screen_res.0 as f32 / screen_res.1 as f32, near, pos, dir);
    let mx_ref: &[f32] = mx_total.as_slice();

    let temp_buf = device
        .create_buffer_mapped(16, wgpu::BufferUsage::COPY_SRC)
        .fill_from_slice(mx_ref);

    encoder.copy_buffer_to_buffer(&temp_buf, 0, uniform_buf, 0, 64);
    //View
    let mx_total = create_view(pos, dir);
    let mx_ref: &[f32] = mx_total.as_slice();

    let temp_buf = device
        .create_buffer_mapped(16, wgpu::BufferUsage::COPY_SRC)
        .fill_from_slice(mx_ref);

    encoder.copy_buffer_to_buffer(&temp_buf, 0, uniform_buf, 64, 64);
    //Proj
    let mx_total = create_proj(screen_res.0 as f32 / screen_res.1 as f32, near);
    let mx_ref: &[f32] = mx_total.as_slice();

    let temp_buf = device
        .create_buffer_mapped(16, wgpu::BufferUsage::COPY_SRC)
        .fill_from_slice(mx_ref);

    encoder.copy_buffer_to_buffer(&temp_buf, 0, uniform_buf, 64 * 2, 64);
    //Normal
    let mx_total = create_normal(pos, dir);
    let mx_ref: &[f32] = mx_total.as_slice();

    let temp_buf = device
        .create_buffer_mapped(16, wgpu::BufferUsage::COPY_SRC)
        .fill_from_slice(mx_ref);

    encoder.copy_buffer_to_buffer(&temp_buf, 0, uniform_buf, 64 * 3, 64);
}

impl App {
    pub fn rts_camera(&mut self, sim_sec: f32) {
        use winit::event::VirtualKeyCode as Key;
        let key_pressed = &self.input_state.key_pressed;
        let on = |vkc| key_pressed.contains(&vkc);

        let mut offset = Vector3::new(0.0, 0.0, 0.0);
        let mut dir_offset = self.game_state.dir.clone();
        let mut new_dir = None;

        let camera_ground_height = self.heightmap_gpu.phy.z(
            self.game_state
                .position
                .x
                .max(0.0)
                .min(self.heightmap_gpu.phy.width as f32 - 1.0),
            self.game_state
                .position
                .y
                .max(0.0)
                .min(self.heightmap_gpu.phy.height as f32 - 1.0),
        );
        let height_from_ground = self.game_state.position.z - camera_ground_height;
        let distance_camera_middle_screen = self
            .game_state
            .screen_center_world_pos
            .map(|scwp| (self.game_state.position.coords - scwp).magnitude())
            .unwrap_or(height_from_ground);
        let k =
            (if !on(Key::LShift) { 1.0 } else { 2.0 }) * distance_camera_middle_screen.max(10.0);
        //Game
        if on(Key::S) {
            offset.y -= k;
        }
        if on(Key::Z) {
            offset.y += k;
        }
        if on(Key::Q) {
            offset.x -= k;
        }
        if on(Key::D) {
            offset.x += k;
        }

        if on(Key::LControl) {
            if let Some(screen_center_world_pos) = self.game_state.screen_center_world_pos {
                if self.input_state.last_scroll != 0.0 {
                    let camera_to_center =
                        screen_center_world_pos - self.game_state.position.coords;

                    let distance = camera_to_center.norm();

                    let mut new_camera_to_center = camera_to_center.normalize();

                    if self.input_state.last_scroll > 0.0 {
                        new_camera_to_center.y += 1.0 * 0.30;
                    }
                    if self.input_state.last_scroll < 0.0 {
                        new_camera_to_center.z -= 1.0 * 0.30;
                    }
                    new_camera_to_center.x = 0.0;

                    new_camera_to_center = new_camera_to_center.normalize();
                    new_camera_to_center.y = new_camera_to_center.y.max(0.01);

                    new_dir = Some(new_camera_to_center);
                    let new_pos =
                        screen_center_world_pos - new_camera_to_center.normalize() * distance;
                    offset += (new_pos - self.game_state.position.coords) / sim_sec;
                }
            } else {
                if self.input_state.last_scroll > 0.0 {
                    dir_offset.y += 0.010 / sim_sec;
                }
                if self.input_state.last_scroll < 0.0 {
                    dir_offset.z -= 0.010 / sim_sec;
                }
            }
        } else {
            if let Some(mouse_world_pos) = self.game_state.mouse_world_pos {
                let u = (mouse_world_pos - self.game_state.position.coords).normalize();
                offset += self.input_state.last_scroll * u * k * 0.75 * 0.320 / sim_sec;
            } else {
                offset.z = -self.input_state.last_scroll * k * 0.75 * 0.20 / sim_sec;
            }
        }

        self.game_state.position += offset * sim_sec;
        self.game_state.dir = (self.game_state.dir + dir_offset * 33.0 * sim_sec).normalize();

        new_dir.map(|new_dir| {
            self.game_state.dir = new_dir;
        });

        self.game_state.position.z = self.game_state.position.z.max(camera_ground_height + 3.0);

        self.game_state.position_smooth += (self.game_state.position.coords
            - self.game_state.position_smooth.coords)
            * sim_sec.min(0.033)
            * 15.0;

        self.game_state.dir_smooth +=
            (self.game_state.dir - self.game_state.dir_smooth) * sim_sec.min(0.033) * 15.0;
    }

    pub fn orbit_camera(&mut self, sim_sec: f32) {
        let to_orbit =
            (self.unit_editor.orbit.coords - self.game_state.position_smooth.coords).normalize();

        let right_vec = to_orbit.cross(&Vector3::new(0.0, 0.0, 1.0)).normalize();
        let up_vec = -to_orbit.cross(&right_vec).normalize();
        if self
            .input_state
            .mouse_pressed
            .contains(&winit::event::MouseButton::Middle)
        {
            let k = 3.0;
            let right_offset = -self.input_state.cursor_offset.0 as f32 * right_vec * sim_sec * k;
            let up_offset = self.input_state.cursor_offset.1 as f32 * up_vec * sim_sec * k;
            self.game_state.position += right_offset + up_offset;
            self.game_state.position_smooth += right_offset + up_offset;

            if self
                .input_state
                .key_pressed
                .contains(&winit::event::VirtualKeyCode::LShift)
            {
                self.unit_editor.orbit += right_offset + up_offset;
            }
        }

        let to_orbit =
            (self.unit_editor.orbit.coords - self.game_state.position_smooth.coords).normalize();

        self.game_state.dir = to_orbit;
        self.game_state.dir_smooth = self.game_state.dir;

        self.game_state.position += self.input_state.last_scroll * to_orbit * sim_sec * 15.0;
        self.game_state.position_smooth = self.game_state.position;
    }
}

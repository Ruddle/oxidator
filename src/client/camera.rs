extern crate nalgebra as na;
use na::{Matrix4, Point3, Vector3};

pub fn create_view_proj(aspect_ratio: f32, pos: &Point3<f32>, dir: &Vector3<f32>) -> Matrix4<f32> {
    let mx_projection = Matrix4::new_perspective(aspect_ratio, 45f32, 1.0, 10.0);

    let mx_view = Matrix4::look_at_rh(pos, &(pos + dir), &Vector3::new(0.0, 0.0, 1.0));

    let mx_correction: Matrix4<f32> = Matrix4::new(
        1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
    );
    mx_correction * mx_projection * mx_view
}

pub fn update_camera_uniform(
    screen_res: (u32, u32),
    pos: &Point3<f32>,
    dir: &Vector3<f32>,
    uniform_buf: &wgpu::Buffer,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
) {
    let mx_total = create_view_proj(screen_res.0 as f32 / screen_res.1 as f32, pos, dir);
    let mx_ref: &[f32] = mx_total.as_slice();

    let temp_buf = device
        .create_buffer_mapped(16, wgpu::BufferUsage::COPY_SRC)
        .fill_from_slice(mx_ref);

    encoder.copy_buffer_to_buffer(&temp_buf, 0, uniform_buf, 0, 64);
}

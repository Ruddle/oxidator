#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    _pos: [f32; 2],
}

fn z(x: f32, y: f32) -> f32 {
    30.0 * f32::sin((x + y) / 95.0)
        + 15.0 * (f32::sin(x / 20.0) * f32::cos(y / 45.0 + 1.554))
        + 3.0 * (f32::sin(x / 3.0 + f32::sin(x / 12.0)) * f32::cos(y / 3.3 + 1.94))
    //        + 1.0 * (f32::sin(x * 3.0) * f32::cos(y * 3.3 + 1.94))
}

pub fn create_texels(width: u32, height: u32, t: f32) -> Vec<f32> {
    let mut texels = Vec::new();
    for i in 0..width {
        for j in 0..height {
            texels.push(z(i as f32 + t, j as f32 + t));
            texels.push(z(i as f32 + t, j as f32 + t));
            texels.push(z(i as f32 + t, j as f32 + t));
            texels.push(z(i as f32 + t, j as f32 + t));
        }
    }

    texels
}

pub fn create_vertices(width: u32, height: u32, t: f32) -> Vec<Vertex> {
    let nb_square = ((width - 1) * (height - 1)) as usize;
    let mut vertex_data = Vec::with_capacity(nb_square * 4);

    for j in 0..height {
        for i in 0..width {
            vertex_data.push(Vertex {
                _pos: [i as f32, j as f32],
            });
        }
    }
    vertex_data
}

pub fn create_indices(width: u32, height: u32) -> Vec<u32> {
    let nb_square = ((width - 1) * (height - 1)) as usize;

    let mut index_data = Vec::with_capacity(nb_square * 4);

    let index_of = |i, j| -> u32 { i + j * width };

    for i in 0_u32..width - 1 {
        for j in 0_u32..height - 1 {
            let a: u32 = index_of(i, j);
            let b: u32 = index_of(i + 1, j);
            let c: u32 = index_of(i + 1, j + 1);
            let d: u32 = index_of(i, j + 1);

            if (i + j) % 2 == 0 {
                index_data.push(a);
                index_data.push(b);
                index_data.push(c);
                index_data.push(a);
                index_data.push(c);
                index_data.push(d);
            } else {
                index_data.push(a);
                index_data.push(b);
                index_data.push(d);
                index_data.push(b);
                index_data.push(c);
                index_data.push(d);
            }
        }
    }

    index_data
}

pub fn create_vertices_indices(width: u32, height: u32, t: f32) -> (Vec<Vertex>, Vec<u32>) {
    use std::time::Instant;
    let start = Instant::now();
    let mut vertex_data = create_vertices(width, height, t);
    println!("create_indices took {}us", start.elapsed().as_micros());

    let start = Instant::now();
    let mut index_data = create_indices(width, height);
    println!("create_indices took {}us", start.elapsed().as_micros());

    //    println!("index_data size  {}", index_data.len());
    //    println!("vertex_data size {}", vertex_data.len());

    (vertex_data, index_data)
}

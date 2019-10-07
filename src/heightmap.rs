#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    _pos: [f32; 3],
}
pub const CHUNK_SIZE: u32 = 16;

fn z(x: f32, y: f32) -> f32 {
    30.0 * f32::sin((x + y) / 95.0)
        + 15.0 * (f32::sin(x / 20.0) * f32::cos(y / 45.0 + 1.554))
        + 3.0 * (f32::sin(x / 3.0) * f32::cos(y / 3.3 + 1.94))
}

pub fn create_vertices(width_n: u32, height_n: u32, t: f32) -> Vec<Vertex> {
    let width = CHUNK_SIZE * width_n;
    let height = CHUNK_SIZE * height_n;

    let nb_square = ((width - 1) * (height - 1)) as usize;
    let mut vertex_data = Vec::with_capacity(nb_square * 4);

    for chunk_j in 0..height_n {
        for chunk_i in 0..width_n {
            for j in 0_u32..CHUNK_SIZE {
                for i in 0_u32..CHUNK_SIZE {
                    let (x, y) = (
                        (i + chunk_i * CHUNK_SIZE) as f32,
                        (j + chunk_j * CHUNK_SIZE) as f32,
                    );
                    vertex_data.push(Vertex {
                        _pos: [x, y, z(x + t, y + t)],
                    });
                }
            }
        }
    }
    vertex_data
}

pub fn create_indices(width_n: u32, height_n: u32) -> Vec<u32> {
    let width = CHUNK_SIZE * width_n;
    let height = CHUNK_SIZE * height_n;
    let nb_square = ((width - 1) * (height - 1)) as usize;

    let mut index_data = Vec::with_capacity(nb_square * 4);

    let index_of = |i: u32, j: u32| -> u32 {
        let chunk_i = i / CHUNK_SIZE;
        let chunk_j = j / CHUNK_SIZE;
        let chunk_number = chunk_i + chunk_j * width_n;
        let di = i % CHUNK_SIZE;
        let dj = j % CHUNK_SIZE;

        chunk_number * CHUNK_SIZE * CHUNK_SIZE + di + dj * CHUNK_SIZE
    };

    for i in 0_u32..width - 1 {
        for j in 0_u32..height - 1 {
            let a: u32 = index_of(i, j);
            let b: u32 = index_of(i + 1, j);
            let c: u32 = index_of(i + 1, j + 1);
            let d: u32 = index_of(i, j + 1);
            index_data.push(a);
            index_data.push(b);
            index_data.push(c);
            index_data.push(a);
            index_data.push(c);
            index_data.push(d);
        }
    }
    index_data
}

pub fn create_vertices_indices(width_n: u32, height_n: u32, t: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertex_data = create_vertices(width_n, height_n, t);
    let mut index_data = create_indices(width_n, height_n);

    //    println!("index_data size  {}", index_data.len());
    //    println!("vertex_data size {}", vertex_data.len());

    (vertex_data, index_data)
}

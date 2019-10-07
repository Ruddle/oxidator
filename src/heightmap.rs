#[derive(Clone, Copy)]
pub struct Vertex {
    _pos: [f32; 4],
}
pub const CHUNK_SIZE: u32 = 16;

pub fn create_vertices(width_n: u32, height_n: u32) -> (Vec<Vertex>, Vec<u32>) {
    let width = CHUNK_SIZE * width_n;
    let height = CHUNK_SIZE * height_n;

    let nb_square = ((width - 1) * (height - 1)) as usize;
    let mut vertex_data = Vec::with_capacity(nb_square * 4);
    let mut index_data = Vec::with_capacity(nb_square * 4);

    let mut last_index = 0;
    //    for i in 0_u32..size - 1 {
    //        for j in 0_u32..size - 1 {
    //            let vertex = |x: f32, y: f32| -> Vertex {
    //                Vertex {
    //                    _pos: [x, y, 5.0 * (f32::sin(x / 3.0) * y) / 1000.0, 0.0],
    //                    _tex_coord: [x / size as f32, y / size as f32],
    //                }
    //            };
    //
    //            let index_a: u32 = last_index;
    //            let a = vertex(i as f32, j as f32);
    //            let b = vertex(i as f32 + 1.0, j as f32);
    //            let c = vertex(i as f32 + 1.0, j as f32 + 1.0);
    //            let d = vertex(i as f32, j as f32 + 1.0);
    //
    //            vertex_data.push(a);
    //            vertex_data.push(b);
    //            vertex_data.push(c);
    //            vertex_data.push(d);
    //
    //            index_data.push(index_a);
    //            index_data.push(index_a + 1);
    //            index_data.push(index_a + 2);
    //            index_data.push(index_a);
    //            index_data.push(index_a + 2);
    //            index_data.push(index_a + 3);
    //
    //            last_index = index_a + 3 + 1;
    //        }
    //    }
    //    println!("last_index {}", last_index);

    fn z(x: f32, y: f32) -> f32 {
        30.0 * f32::sin((x + y) / 95.0)
            + 15.0 * (f32::sin(x / 20.0) * f32::cos(y / 45.0 + 1.554))
            + 3.0 * (f32::sin(x / 3.0) * f32::cos(y / 3.3 + 1.94))
    }

    for chunk_j in 0..height_n {
        for chunk_i in 0..width_n {
            for j in 0_u32..CHUNK_SIZE {
                for i in 0_u32..CHUNK_SIZE {
                    let vertex = |x: f32, y: f32| -> Vertex {
                        Vertex {
                            _pos: [x, y, z(x, y), 0.0],
                        }
                    };

                    let a = vertex(
                        (i + chunk_i * CHUNK_SIZE) as f32,
                        (j + chunk_j * CHUNK_SIZE) as f32,
                    );
                    vertex_data.push(a);
                }
            }
        }
    }

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

    println!("index_data size  {}", index_data.len());
    println!("vertex_data size {}", vertex_data.len());

    //    for chunk in index_data.chunks(3) {
    //        if let &[a, b, c] = chunk {
    //            let va = vertex_data[a as usize];
    //            let vb = vertex_data[b as usize];
    //            let vc = vertex_data[c as usize];
    //            println!("Index    : {:?} {:?} {:?}", a, b, c);
    //            println!("Triangle : {:?} {:?} {:?}", va._pos, vb._pos, vc._pos)
    //        } else {
    //            println!("ERROR chunk");
    //        }
    //    }

    (vertex_data, index_data)
}

use crate::Vertex;

//pub struct Vertex {
//    _pos: [f32; 4],
//    _tex_coord: [f32; 2],
//}

pub const CHUNK_SIZE: u32 = 10;

pub fn create_vertices() -> (Vec<Vertex>, Vec<u32>) {
    let size = 1000_u32;
    let nb_square = ((size - 1) * (size - 1)) as usize;
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
        30.0 * f32::sin((x + y) / 95.0) + 15.0 * (f32::sin(x / 20.0) * f32::cos(y / 45.0 + 1.554))
    }

    for j in 0_u32..size {
        for i in 0_u32..size {
            let vertex = |x: f32, y: f32| -> Vertex {
                Vertex {
                    _pos: [x, y, z(x, y), 0.0],
                    _tex_coord: [x / size as f32, y / size as f32],
                }
            };

            let a = vertex(i as f32, j as f32);
            vertex_data.push(a);
        }
    }

    for i in 0_u32..size - 1 {
        for j in 0_u32..size - 1 {
            let a: u32 = i + j * size;
            let b: u32 = i + 1 + j * size;
            let c: u32 = i + 1 + (j + 1) * size;
            let d: u32 = i + (j + 1) * size;
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

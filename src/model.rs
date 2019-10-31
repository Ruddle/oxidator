#[derive(Clone, Copy)]
pub struct Vertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

#[derive(Clone)]
pub struct TriangleList {
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u32>,
}

pub fn open_obj(path: &str) -> Result<TriangleList, String> {
    use obj::{load_obj, Obj};
    use std::fs::File;
    use std::io::BufReader;

    let input = BufReader::new(File::open(path).unwrap());
    let model: Obj<obj::TexturedVertex> = load_obj(input).map_err(|e| format!("{:?}", e))?;

    let vertex_data: Vec<_> = model
        .vertices
        .iter()
        .map(|v| Vertex {
            _pos: [v.position[0], v.position[1], v.position[2], 1.0],
            _tex_coord: [v.texture[0], v.texture[1]],
        })
        .collect();

    Ok(TriangleList {
        vertex_data,
        index_data: model.indices.iter().map(|u| *u as u32).collect(),
    })
}

pub fn create_cube() -> TriangleList {
    fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
        Vertex {
            _pos: [
                pos[0] as f32 / 2.0,
                pos[1] as f32 / 2.0,
                pos[2] as f32 / 2.0,
                1.0,
            ],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }

    let vertex_data = [
        // top (0, 0, 1)
        vertex([-1, -1, 1], [0, 0]),
        vertex([1, -1, 1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([-1, 1, 1], [0, 1]),
        // bottom (0, 0, -1)
        vertex([-1, 1, -1], [1, 0]),
        vertex([1, 1, -1], [0, 0]),
        vertex([1, -1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // right (1, 0, 0)
        vertex([1, -1, -1], [0, 0]),
        vertex([1, 1, -1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([1, -1, 1], [0, 1]),
        // left (-1, 0, 0)
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, 1, 1], [0, 0]),
        vertex([-1, 1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // front (0, 1, 0)
        vertex([1, 1, -1], [1, 0]),
        vertex([-1, 1, -1], [0, 0]),
        vertex([-1, 1, 1], [0, 1]),
        vertex([1, 1, 1], [1, 1]),
        // back (0, -1, 0)
        vertex([1, -1, 1], [0, 0]),
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, -1, -1], [1, 1]),
        vertex([1, -1, -1], [0, 1]),
    ];

    let index_data: &[u32] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    TriangleList {
        vertex_data: vertex_data.to_vec(),
        index_data: index_data.to_vec(),
    }
}

pub fn create_small_cube() -> TriangleList {
    fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
        Vertex {
            _pos: [
                pos[0] as f32 / 4.0,
                pos[1] as f32 / 4.0,
                pos[2] as f32 / 4.0,
                1.0,
            ],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }

    let vertex_data = [
        // top (0, 0, 1)
        vertex([-1, -1, 1], [0, 0]),
        vertex([1, -1, 1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([-1, 1, 1], [0, 1]),
        // bottom (0, 0, -1)
        vertex([-1, 1, -1], [1, 0]),
        vertex([1, 1, -1], [0, 0]),
        vertex([1, -1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // right (1, 0, 0)
        vertex([1, -1, -1], [0, 0]),
        vertex([1, 1, -1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([1, -1, 1], [0, 1]),
        // left (-1, 0, 0)
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, 1, 1], [0, 0]),
        vertex([-1, 1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // front (0, 1, 0)
        vertex([1, 1, -1], [1, 0]),
        vertex([-1, 1, -1], [0, 0]),
        vertex([-1, 1, 1], [0, 1]),
        vertex([1, 1, 1], [1, 1]),
        // back (0, -1, 0)
        vertex([1, -1, 1], [0, 0]),
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, -1, -1], [1, 1]),
        vertex([1, -1, -1], [0, 1]),
    ];

    let index_data: &[u32] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    TriangleList {
        vertex_data: vertex_data.to_vec(),
        index_data: index_data.to_vec(),
    }
}

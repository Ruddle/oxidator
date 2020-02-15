#[derive(Clone, Copy)]
pub struct Vertex {
    _pos: [f32; 4],
    _nor: [f32; 3],
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

    let input = BufReader::new(File::open(path).expect(&format!("Can't open {}", path)));
    let model: Obj<obj::TexturedVertex> = load_obj(input).map_err(|e| format!("{:?}", e))?;

    let vertex_data: Vec<_> = model
        .vertices
        .iter()
        .map(|v| Vertex {
            _pos: [v.position[0], v.position[1], v.position[2], 1.0],
            _nor: [v.normal[0], v.normal[1], v.normal[2]],
            _tex_coord: [v.texture[0], v.texture[1]],
        })
        .collect();

    Ok(TriangleList {
        vertex_data,
        index_data: model.indices.iter().map(|u| *u as u32).collect(),
    })
}

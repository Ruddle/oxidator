use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::hash::Hasher;
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    _pos: [f32; 2],
}

impl Vertex {
    fn is_same(&self, other: &Self) -> bool {
        self._pos[0] == other._pos[0] && self._pos[1] == other._pos[1]
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Vertex) -> bool {
        self.canonicalize() == other.canonicalize()
    }
}

impl Eq for Vertex {}

impl Vertex {
    fn canonicalize(&self) -> i128 {
        (self._pos[0] * 1024.0 * 1024.0).round() as i128
            + (self._pos[1] * 1024.0 * 1024.0 * 1024.0 * 1024.0).round() as i128
    }
}
impl Hash for Vertex {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.canonicalize().hash(state);
    }
}

pub fn z(x: f32, y: f32) -> f32 {
    30.0 * f32::sin((x + y) / 95.0)
        + 15.0 * (f32::sin(x / 20.0) * f32::cos(y / 45.0 + 1.554))
        + 3.0 * (f32::sin(x / 3.0 + f32::sin(x / 12.0)) * f32::cos(y / 3.3 + 1.94))
    //        + 1.0 * (f32::sin(x * 3.0) * f32::cos(y * 3.3 + 1.94))
}

pub fn create_texels(width: u32, height: u32, t: f32) -> Vec<f32> {
    let mut texels = Vec::with_capacity((width * height) as usize);
    for j in 0..height {
        for i in 0..width {
            texels.push(z(i as f32 + t, j as f32 + t));
        }
    }

    texels
}

pub fn create_vertex_index_rings(hsize: u32) -> (Vec<Vertex>, Vec<u32>) {
    let nb_square = ((hsize - 1) * (hsize - 1)) as usize;
    let mut vertex_data = Vec::with_capacity(nb_square * 4);
    let mut index_data = Vec::with_capacity(nb_square * 4);

    let vertex = |x: f32, y: f32, bx: f32, by: f32| -> Vertex {
        Vertex {
            _pos: [x * 0.5, y * 0.5],
        }
    };

    for i in 0_u32..hsize {
        for j in 0_u32..hsize {
            let index_a: u32 = vertex_data.len() as u32;
            let a = vertex(i as f32, j as f32, 0.0, 0.0);
            let b = vertex(i as f32 + 1.0, j as f32, 0.1, 0.0);
            let c = vertex(i as f32 + 1.0, j as f32 + 1.0, 1.0, 1.0);
            let d = vertex(i as f32, j as f32 + 1.0, 0.0, 1.0);
            vertex_data.push(a);
            vertex_data.push(b);
            vertex_data.push(c);
            vertex_data.push(d);
            index_data.push(index_a);
            index_data.push(index_a + 1);
            index_data.push(index_a + 2);
            index_data.push(index_a);
            index_data.push(index_a + 2);
            index_data.push(index_a + 3);
        }
    }

    println!("{}", vertex_data.len());

    enum Pass {
        Step(i32),
        Trans { from: i32, to: i32 },
    }

    let mut passes = vec![];
    passes.push(Pass::Trans { from: 1, to: 2 });
    passes.extend((0..31).into_iter().map(|e| Pass::Step(2)));
    passes.push(Pass::Trans { from: 2, to: 4 });
    passes.extend((0..31).into_iter().map(|e| Pass::Step(4)));
    passes.push(Pass::Trans { from: 4, to: 8 });
    passes.extend((0..31).into_iter().map(|e| Pass::Step(8)));
    passes.push(Pass::Trans { from: 8, to: 16 });
    passes.extend((0..127).into_iter().map(|e| Pass::Step(16)));
    passes.push(Pass::Trans { from: 16, to: 32 });
    passes.extend((0..170).into_iter().map(|e| Pass::Step(32)));

    let mut start_min = hsize as i32;
    for pass in passes.iter() {
        match pass {
            Pass::Trans { from, to } => {
                println!("Pass::Trans {} {}", from, to);

                let i = start_min;
                let j = start_min;
                {
                    let index_a: u32 = vertex_data.len() as u32;
                    let a = vertex(i as f32, j as f32, 0.0, 0.0);
                    let b = vertex(i as f32 + *to as f32, j as f32, 0.1, 0.0);
                    let c = vertex(i as f32 + *to as f32, j as f32 + *to as f32, 1.0, 1.0);
                    let d = vertex(i as f32, j as f32 + *to as f32, 0.0, 1.0);
                    vertex_data.push(a);
                    vertex_data.push(b);
                    vertex_data.push(c);
                    vertex_data.push(d);
                    index_data.push(index_a);
                    index_data.push(index_a + 1);
                    index_data.push(index_a + 2);
                    index_data.push(index_a);
                    index_data.push(index_a + 2);
                    index_data.push(index_a + 3);
                };

                let i = start_min;
                for j in (0..=start_min - *to).step_by(*to as usize) {
                    let index_a: u32 = vertex_data.len() as u32;
                    let a = vertex(i as f32, j as f32, 0.0, 0.0);
                    let b = vertex(i as f32 + *to as f32, j as f32, 0.1, 0.0);
                    let c = vertex(i as f32 + *to as f32, j as f32 + *to as f32, 1.0, 1.0);
                    let d = vertex(i as f32, j as f32 + *to as f32, 0.0, 1.0);
                    let e = vertex(i as f32, j as f32 + *from as f32, 0.0, 0.5);
                    vertex_data.push(a);
                    vertex_data.push(b);
                    vertex_data.push(c);
                    vertex_data.push(d);
                    vertex_data.push(e);
                    index_data.push(index_a + 4);
                    index_data.push(index_a);
                    index_data.push(index_a + 1);
                    index_data.push(index_a + 4);
                    index_data.push(index_a + 1);
                    index_data.push(index_a + 2);
                    index_data.push(index_a + 4);
                    index_data.push(index_a + 2);
                    index_data.push(index_a + 3);
                }

                let j = start_min;
                for i in (0..=start_min - *to).step_by(*to as usize) {
                    let index_a: u32 = vertex_data.len() as u32;
                    let a = vertex(i as f32, j as f32, 0.0, 0.0);
                    let b = vertex(i as f32 + *to as f32, j as f32, 0.1, 0.0);
                    let c = vertex(i as f32 + *to as f32, j as f32 + *to as f32, 1.0, 1.0);
                    let d = vertex(i as f32, j as f32 + *to as f32, 0.0, 1.0);
                    let e = vertex(i as f32 + *from as f32, j as f32, 0.5, 0.0);
                    vertex_data.push(a);
                    vertex_data.push(b);
                    vertex_data.push(c);
                    vertex_data.push(d);
                    vertex_data.push(e);
                    index_data.push(index_a + 4);
                    index_data.push(index_a + 1);
                    index_data.push(index_a + 2);

                    index_data.push(index_a + 4);
                    index_data.push(index_a + 2);
                    index_data.push(index_a + 3);
                    index_data.push(index_a + 4);
                    index_data.push(index_a + 3);
                    index_data.push(index_a);
                }
                start_min += *to;
            }

            Pass::Step(step) => {
                let mut make_square = |i, j, step| {
                    let index_a: u32 = vertex_data.len() as u32;
                    let a = vertex(i as f32, j as f32, 0.0, 0.0);
                    let b = vertex(i as f32 + step as f32, j as f32, 0.1, 0.0);
                    let c = vertex(i as f32 + step as f32, j as f32 + step as f32, 1.0, 1.0);
                    let d = vertex(i as f32, j as f32 + step as f32, 0.0, 1.0);
                    {
                        vertex_data.push(a);
                        vertex_data.push(b);
                        vertex_data.push(c);
                        vertex_data.push(d);
                        index_data.push(index_a);
                        index_data.push(index_a + 1);
                        index_data.push(index_a + 2);
                        index_data.push(index_a);
                        index_data.push(index_a + 2);
                        index_data.push(index_a + 3);
                    }
                };

                let j = start_min;
                for i in (0..=start_min).step_by(*step as usize) {
                    make_square(i, j, *step);
                }

                let i = start_min;
                for j in (0..start_min).step_by(*step as usize) {
                    make_square(i, j, *step);
                }
                start_min += *step;
            }
        }
    }

    println!("Passes Done");
    println!("index_data size  {}", index_data.len());
    println!("vertex_data size {}", vertex_data.len());

    let (mut vertex_data, mut index_data) = optimize_vertex_index(vertex_data, index_data);

    for (i_mul, j_mul) in [(-1.0_f32, 1.0_f32), (1.0, -1.0)].iter() {
        let mut symmetry_vertex_data = Vec::new();

        for &vert in vertex_data.iter() {
            symmetry_vertex_data.push(Vertex {
                _pos: [i_mul * vert._pos[0], j_mul * vert._pos[1]],
            });
        }

        let copie: Vec<u32> = index_data.iter().copied().collect();
        let symmetry_index_data: Vec<u32> = copie
            .chunks(3)
            .into_iter()
            .flat_map(|e| vec![e[1], e[0], e[2]])
            .map(|i| i + vertex_data.len() as u32)
            .collect();

        vertex_data.extend(symmetry_vertex_data);
        index_data.extend(symmetry_index_data);
    }

    println!("Symmetry Done");
    println!("index_data size  {}", index_data.len());
    println!("vertex_data size {}", vertex_data.len());
    (vertex_data, index_data)
}

pub fn optimize_vertex_index(
    vertex_data: Vec<Vertex>,
    mut index_data: Vec<u32>,
) -> (Vec<Vertex>, Vec<u32>) {
    let start = std::time::Instant::now();

    println!("Before Optimisation");
    println!("index_data size  {}", index_data.len());
    println!("vertex_data size {}", vertex_data.len());
    let mut new_vertex_data: Vec<Vertex> = Vec::new();

    let mut map: HashMap<Vertex, Option<usize>> = HashMap::new();

    for v in &vertex_data {
        map.insert(v.clone(), None);
    }

    for i in index_data.iter_mut() {
        let v = &vertex_data[*i as usize];

        if let Some(position) = map.get(v).unwrap() {
            *i = *position as u32;
        } else {
            new_vertex_data.push(v.clone());
            let new_index = new_vertex_data.len() - 1;
            map.insert(v.clone(), Some(new_index));
            *i = new_index as u32;
        }
    }

    println!("Optimisation Done");
    println!("index_data size  {}", index_data.len());
    println!("vertex_data size {}", new_vertex_data.len());
    println!("Optimisation took {}us", start.elapsed().as_micros());

    (new_vertex_data, index_data)
}

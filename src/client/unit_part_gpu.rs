use crate::gpu_obj;
use crate::model;
use gpu_obj::model_gpu::ModelGpu;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
pub enum ModelGpuState {
    ToLoad(model::TriangleList),
    Ready(ModelGpu),
    Error(String),
}

pub struct UnitPartGpu {
    pub states: Vec<ModelGpuState>,
    path_to_index: HashMap<PathBuf, usize>,
}

impl UnitPartGpu {
    pub fn new() -> Self {
        UnitPartGpu {
            states: Vec::new(),
            path_to_index: HashMap::new(),
        }
    }

    pub fn index_of(&self, path: PathBuf) -> Option<&usize> {
        self.path_to_index.get(&path)
    }

    pub fn path_get(&self, path: PathBuf) -> Option<&ModelGpuState> {
        self.index_of(path).map(|index| &self.states[*index])
    }

    pub fn path_get_mut(&mut self, path: PathBuf) -> Option<&mut ModelGpuState> {
        self.index_of(path)
            .cloned()
            .map(move |index| &mut self.states[index])
    }

    pub fn get(&self, index: usize) -> &ModelGpuState {
        &self.states[index]
    }
    pub fn get_mut(&mut self, index: usize) -> &mut ModelGpuState {
        &mut self.states[index]
    }

    fn load_at(&mut self, index: usize, path: PathBuf) {
        self.path_to_index.insert(path.clone(), index);
        let to_push = match crate::model::open_obj(path.to_str().unwrap()) {
            Ok(triangle_list) => ModelGpuState::ToLoad(triangle_list.clone()),
            Err(e) => ModelGpuState::Error(e),
        };
        self.states.push(to_push);
    }
    ///Push a new entry, regardless of if the same path is already present.
    ///Returns the entry index
    pub fn append(&mut self, path: PathBuf) -> usize {
        let index = self.states.len();
        self.load_at(index, path);
        index
    }

    pub fn reload(&mut self, path: PathBuf) -> usize {
        match self.index_of(path.clone()).cloned() {
            Some(index) => {
                self.load_at(index, path);
                index
            }
            None => self.append(path),
        }
    }

    pub fn index_of_or_create_if_na(&mut self, path: PathBuf) -> usize {
        match self.index_of(path.clone()) {
            Some(index) => *index,
            None => self.append(path),
        }
    }

    pub fn path_get_or_create_if_na(&mut self, path: PathBuf) -> (usize, &ModelGpuState) {
        let index = self.index_of_or_create_if_na(path);
        (index, self.get(index))
    }
}

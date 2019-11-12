use super::client::*;
use crate::model::*;
use crate::utils::FileTree;
use crate::*;
use gpu_obj::model_gpu::ModelGpu;
use na::{Matrix4, Point3, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedMesh {
    pub position: Point3<f32>,
    pub dir: Vector3<f32>,
    pub mesh_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlacedCollider {
    Sphere { position: Point3<f32>, radius: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Joint {
    Fix,
    Spherical,
}

impl Joint {
    pub fn next(&self) -> Self {
        match self {
            Joint::Fix => Joint::Spherical,
            Joint::Spherical => Joint::Fix,
        }
    }

    pub fn replace_with_next(&mut self) {
        let next = self.next();
        std::mem::replace(self, next);
    }
}

#[derive(Debug, Clone, typename::TypeName, PartialEq, Serialize, Deserialize)]
pub struct PartTree {
    pub id: utils::Id<PartTree>,
    pub placed_mesh: Option<PlacedMesh>,
    pub placed_collider: Option<PlacedCollider>,
    pub joint: Joint,
    pub children: Vec<PartTree>,
}

impl PartTree {
    pub fn find_node(&mut self, id: utils::Id<PartTree>) -> Option<&mut PartTree> {
        if self.id == id {
            Some(self)
        } else {
            for c in self.children.iter_mut() {
                match c.find_node(id) {
                    Some(node) => return Some(node),
                    None => {}
                }
            }
            None
        }
    }

    ///Remove a node and return the parent if successful
    pub fn remove_node(&mut self, id: utils::Id<PartTree>) -> Option<utils::Id<PartTree>> {
        let pos = self.children.iter().position(|e| e.id == id);
        match pos {
            Some(index) => {
                self.children.remove(index);
                Some(self.id)
            }
            None => {
                let mut res = None;
                for c in self.children.iter_mut() {
                    res = res.or(c.remove_node(id));
                }
                res
            }
        }
    }
}

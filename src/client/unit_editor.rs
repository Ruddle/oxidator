use super::client::*;
use crate::model::*;
use crate::utils::FileTree;
use crate::*;
use gpu_obj::model_gpu::ModelGpu;
use na::{Matrix4, Point3, Vector2, Vector3};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct DisplayModel {
    pub position: Point3<f32>,
    pub dir: Vector3<f32>,
    pub model_path: PathBuf,
}

pub struct UnitEditor {
    pub orbit: Point3<f32>,
    pub root: PartTree,
    pub asset_dir_cached: FileTree,
    pub selected_id: utils::Id<PartTree>,
}
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, typename::TypeName)]
pub struct PartTree {
    pub id: utils::Id<PartTree>,
    pub dmodel: Option<DisplayModel>,
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

impl UnitEditor {
    pub fn new() -> Self {
        let root = PartTree {
            id: utils::rand_id(),
            children: Vec::new(),
            dmodel: None,
            joint: Joint::Fix,
        };
        UnitEditor {
            orbit: Point3::new(300.0, 100.0, 0.5),
            asset_dir_cached: FileTree::Unknown,
            selected_id: root.id,
            root,
        }
    }

    fn open_obj(
        &mut self,
        path: &Path,
        generic_gpu: &mut HashMap<PathBuf, GenericGpuState>,
    ) -> bool {
        log::debug!("open_obj {:?}", path);
        match crate::model::open_obj(path.to_str().unwrap()) {
            Ok(triangle_list) => {
                generic_gpu.insert(
                    path.to_owned(),
                    GenericGpuState::ToLoad(triangle_list.clone()),
                );
                return true;
            }
            Err(e) => {
                log::warn!("Can't load {:?}: {:?}", path, e);
                generic_gpu.insert(path.to_owned(), GenericGpuState::Error(e));
                return false;
            }
        }
    }

    fn add_to_parts(&mut self, path: PathBuf) {
        log::debug!("adding {:?} to {}", path, self.selected_id);

        match self.root.find_node(self.selected_id) {
            Some(node) => node.children.push(PartTree {
                dmodel: Some(DisplayModel {
                    position: self.orbit.clone(),
                    dir: Vector3::new(1.0, 0.0, 0.0),
                    model_path: path,
                }),
                joint: Joint::Fix,
                id: utils::rand_id(),
                children: Vec::new(),
            }),
            None => {}
        }
    }
}

impl App {
    pub fn init_unit_editor(&mut self) {
        self.clear_gpu_instance_and_game_state();
        self.game_state.position = Point3::new(300.0, 97.0, 1.0);
        self.game_state.position_smooth = Point3::new(300.0, 97.0, 1.0);
        self.game_state.dir = Vector3::new(0.0, 1.0, -1.0);
        self.game_state.dir_smooth = Vector3::new(0.0, 1.0, -1.0);
    }

    pub fn draw_unit_editor_ui(
        ui: &Ui,
        unit_editor: &mut UnitEditor,
        generic_gpu: &mut HashMap<PathBuf, GenericGpuState>,
    ) {
        let path = std::path::Path::new("./src/asset/");

        if let FileTree::Unknown = unit_editor.asset_dir_cached {
            log::debug!("Reading all assets to build file cache");
            unit_editor.asset_dir_cached = FileTree::new(path.to_owned());
        }

        let window = imgui::Window::new(im_str!("Unit Editor"));
        window
            .size([400.0, 600.0], imgui::Condition::FirstUseEver)
            .position([3.0, 115.0], imgui::Condition::FirstUseEver)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .build(&ui, || {
                Self::visit_dirs(
                    &unit_editor.asset_dir_cached.clone(),
                    ui,
                    unit_editor,
                    generic_gpu,
                );

                ui.separator();

                Self::ui_part_tree(ui, &mut unit_editor.root.clone(), unit_editor, true);
            });
    }

    fn ui_part_tree(ui: &Ui, part_tree: &PartTree, unit_editor: &mut UnitEditor, is_root: bool) {
        if unit_editor.selected_id == part_tree.id {
            ui.text(im_str!("Selected"));
        } else {
            if ui.small_button(im_str!("select##{:?}", part_tree.id).as_ref()) {
                unit_editor.selected_id = part_tree.id;
            }
        }
        if !is_root {
            ui.same_line(0.0);
            if ui.small_button(im_str!("remove##{:?}", part_tree.id).as_ref()) {
                let deleter = unit_editor.root.remove_node(part_tree.id);
                if part_tree.id == unit_editor.selected_id {
                    for d in deleter.iter() {
                        unit_editor.selected_id = *d;
                    }
                }
            }
        }
        ui.tree_node(im_str!("children").as_ref())
            .default_open(true)
            .build(|| {
                for c in part_tree.children.iter() {
                    let name = im_str!("child");
                    ChildWindow::new(name)
                        .border(true)
                        .always_auto_resize(true)
                        .build(ui, || {
                            ui.tree_node(im_str!("child").as_ref())
                                .default_open(true)
                                .build(|| {
                                    if let Some(model) = &c.dmodel {
                                        ui.text(im_str!("model {:?}", model.model_path));
                                    }
                                    ui.text(im_str!("joint {:?}", c.joint));
                                    ui.same_line(0.0);
                                    if ui.small_button(im_str!("swap##{:?}", c.id).as_ref()) {
                                        unit_editor
                                            .root
                                            .find_node(c.id)
                                            .unwrap()
                                            .joint
                                            .replace_with_next();
                                    }
                                    Self::ui_part_tree(ui, &c, unit_editor, false);
                                });
                        });
                }
            });
    }

    fn visit_dirs(
        dir: &FileTree,
        ui: &Ui,
        unit_editor: &mut UnitEditor,
        generic_gpu: &mut HashMap<PathBuf, GenericGpuState>,
    ) {
        match dir {
            FileTree::Unknown => {
                ui.text(im_str!("Error reading asset file"));
            }
            FileTree::Leaf { path } => {
                let file_name = path.file_name().unwrap();
                let extension = path.extension().unwrap();
                if extension == "obj" {
                    ui.text(im_str!("{:?}", file_name));
                    ui.same_line(0.0);

                    let state = generic_gpu.get(path);
                    match state {
                        None => {
                            if ui.small_button(im_str!("add to parts##{:?}", path).as_ref()) {
                                log::debug!("add to parts {:?}", path);
                                log::debug!("was not open {:?}", path);
                                if unit_editor.open_obj(&path, generic_gpu) {
                                    unit_editor.add_to_parts(path.clone());
                                }
                            }
                        }
                        Some(GenericGpuState::Ready(_)) | Some(GenericGpuState::ToLoad(_)) => {
                            if ui.small_button(im_str!("add to parts##{:?}", path).as_ref()) {
                                log::debug!("add to parts {:?}", path);
                                unit_editor.add_to_parts(path.clone());
                            }
                            ui.same_line(0.0);
                            if ui.small_button(im_str!("reload##{:?}", path).as_ref()) {
                                unit_editor.open_obj(&path, generic_gpu);
                            }
                        }
                        Some(GenericGpuState::Error(e)) => {
                            ui.text_colored([1.0, 0.0, 0.0, 1.0], im_str!("Error"));
                            ui.same_line(0.0);
                            if ui.small_button(im_str!("reload##{:?}", path).as_ref()) {
                                unit_editor.open_obj(&path, generic_gpu);
                            }
                        }
                    }
                }
            }
            FileTree::Node { path, children } => {
                ui.tree_node(
                    im_str!("{:?}", path.components().last().unwrap().as_os_str()).as_ref(),
                )
                .build(|| {
                    for child in children {
                        Self::visit_dirs(&child, ui, unit_editor, generic_gpu);
                    }
                });
            }
        }
    }
}

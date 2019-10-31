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
    pub parts: Vec<DisplayModel>,
    pub asset_dir_cached: FileTree,
}

impl UnitEditor {
    pub fn new() -> Self {
        UnitEditor {
            orbit: Point3::new(300.0, 100.0, 0.5),
            parts: Vec::new(),
            asset_dir_cached: FileTree::Unknown,
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
                log::debug!("triangle_list");
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
        self.parts.push(DisplayModel {
            position: self.orbit,
            dir: Vector3::new(1.0, 0.0, 0.0),
            model_path: path,
        })
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
            .size([400.0, 300.0], imgui::Condition::FirstUseEver)
            .position([3.0, 415.0], imgui::Condition::FirstUseEver)
            .movable(false)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .build(&ui, || {
                Self::visit_dirs(
                    &unit_editor.asset_dir_cached.clone(),
                    ui,
                    unit_editor,
                    generic_gpu,
                );

                ui.separator();

                ui.tree_node(im_str!("Parts")).default_open(true).build(|| {
                    let parts: Vec<_> = unit_editor.parts.iter().map(|e| e.clone()).collect();
                    for (index, display_model) in parts.iter().enumerate() {
                        ui.text(im_str!("{}: {:?}", index, display_model.model_path));
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("delete##{:?}{:?}", index, path).as_ref()) {
                            unit_editor.parts.remove(index);
                        };
                    }
                });
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

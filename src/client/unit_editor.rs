use super::client::*;
use crate::botdef::BotDef;
use crate::model::*;
use crate::utils::FileTree;
use crate::*;
use gpu_obj::model_gpu::ModelGpu;
use na::{Matrix4, Point3, Vector2, Vector3, Vector4};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use unit::*;
use unit_part_gpu::*;
pub struct UnitEditor {
    pub orbit: Point3<f32>,
    pub botdef: BotDef,
    pub asset_dir_cached: FileTree,
}
impl UnitEditor {
    pub fn new() -> Self {
        let root = PartTree {
            id: utils::rand_id(),
            children: vec![],
            placed_mesh: None,
            placed_collider: None,
            parent_to_self: Matrix4::identity(),
            joint: Joint::Fix,
        };

        let botdef = botdef::BotDef {
            id: utils::rand_id(),
            radius: 0.5,
            max_life: 100,
            turn_accel: 1.5,
            max_turn_rate: 1.5,
            accel: 0.1,
            break_accel: 0.3,
            max_speed: 1.0,
            build_power: 1,
            build_dist: 5.0,
            part_tree: root,
        };

        UnitEditor {
            orbit: Point3::new(300.0, 100.0, 0.5),
            asset_dir_cached: FileTree::Unknown,
            botdef,
        }
    }

    fn add_to_parts(&mut self, parent: utils::Id<PartTree>, path: PathBuf, mesh_index: usize) {
        log::debug!("adding {:?} to {}", path, parent);

        match self.botdef.part_tree.find_node_mut(parent) {
            Some(node) => node.children.push(PartTree {
                placed_mesh: Some(PlacedMesh {
                    trans: utils::face_towards_dir(
                        &Vector3::new(0.0, 0.0, 0.0),
                        &Vector3::new(1.0, 0.0, 0.0),
                        &Vector3::new(0.0, 0.0, 1.0),
                    ),
                    mesh_path: path,
                    mesh_index,
                }),
                placed_collider: None,
                parent_to_self: Matrix4::identity(),
                joint: Joint::Fix,
                id: utils::rand_id(),
                children: vec![],
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
        unit_part_gpu: &mut UnitPartGpu,
    ) {
        let path = std::path::Path::new("./src/asset/");

        if let FileTree::Unknown = unit_editor.asset_dir_cached {
            log::debug!("Reading all assets to build file directory cache");
            unit_editor.asset_dir_cached = FileTree::new(path.to_owned());
        }

        let window = imgui::Window::new(im_str!("Unit Editor"));
        window
            .size([400.0, 600.0], imgui::Condition::FirstUseEver)
            .position([3.0, 115.0], imgui::Condition::FirstUseEver)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .build(&ui, || {
                let BotDef {
                    id,
                    radius,
                    max_life,
                    turn_accel,
                    max_turn_rate,
                    accel,
                    break_accel,
                    max_speed,
                    build_power,
                    build_dist,
                    part_tree,
                } = &unit_editor.botdef;

                let to_rev = 0.5 / std::f32::consts::PI;
                let to_rad = 1.0 / to_rev;
                let to_sec = 10.0;
                let to_frame = 1.0 / to_sec;

                let mut max_turn_rate_human = max_turn_rate * to_sec * to_rev;
                ui.drag_float(
                    im_str!("maximum turn rate (rev/sec)"),
                    &mut max_turn_rate_human,
                )
                .speed(0.01)
                .min(0.01)
                .max(100.0)
                .build();

                let mut turn_accel_human = turn_accel * to_sec * to_sec * to_rev;
                ui.drag_float(
                    im_str!("turn acceleration (rev/sec²)"),
                    &mut turn_accel_human,
                )
                .speed(0.01)
                .min(0.01)
                .max(100.0)
                .build();

                let mut max_speed_human = max_speed * to_sec;
                ui.drag_float(im_str!("max speed (m/sec)"), &mut max_speed_human)
                    .speed(0.01)
                    .min(0.01)
                    .max(100.0)
                    .build();

                let mut accel_human = accel * to_sec * to_sec;
                ui.drag_float(im_str!("acceleration (m/sec²)"), &mut accel_human)
                    .speed(0.01)
                    .min(0.01)
                    .max(100.0)
                    .build();

                let mut break_accel_human = break_accel * to_sec * to_sec;
                ui.drag_float(im_str!("break accel (m/sec²)"), &mut break_accel_human)
                    .speed(0.01)
                    .min(0.01)
                    .max(100.0)
                    .build();

                let mut life = max_life.clone();
                ui.drag_int(im_str!("health"), &mut life).build();

                let mut build_power_ = build_power.clone();
                ui.drag_int(im_str!("build power"), &mut build_power_)
                    .build();

                let mut build_dist_ = build_dist.clone();
                ui.drag_float(im_str!("build distance (m)"), &mut build_dist_)
                    .speed(0.01)
                    .min(0.01)
                    .max(100.0)
                    .build();

                unit_editor.botdef.max_turn_rate = max_turn_rate_human * to_frame * to_rad;
                unit_editor.botdef.turn_accel = turn_accel_human * to_frame * to_frame * to_rad;
                unit_editor.botdef.max_speed = max_speed_human * to_frame;
                unit_editor.botdef.accel = accel_human * to_frame * to_frame;
                unit_editor.botdef.break_accel = break_accel_human * to_frame * to_frame;
                unit_editor.botdef.max_life = life.max(0);
                unit_editor.botdef.build_power = build_power_.max(0);
                unit_editor.botdef.build_dist = build_dist_;
                ui.separator();
                Self::ui_part_tree(
                    ui,
                    &mut unit_editor.botdef.part_tree.clone(),
                    unit_editor,
                    true,
                    unit_part_gpu,
                );

                if ui.button(im_str!("load"), [0.0, 0.0]) {
                    Self::load_botdef_in_editor(
                        "src/asset/botdef/unit_example.json",
                        unit_editor,
                        unit_part_gpu,
                    );
                }
                if ui.button(im_str!("save"), [0.0, 0.0]) {
                    Self::save_botdef_on_disk(
                        &unit_editor.botdef,
                        "src/asset/botdef/unit_example.json",
                    );
                    log::info!("Saving {:?}", unit_editor.botdef.part_tree);
                }
            });
    }

    pub fn save_botdef_on_disk(bot_def: &BotDef, path: &str) {
        use std::fs::OpenOptions;
        use std::io::prelude::*;
        use std::io::{BufReader, BufWriter};
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        let mut buf_w = BufWriter::new(file);
        serde_json::to_writer_pretty(buf_w, bot_def);
        // bincode::serialize_into(buf_w, bot_def);
    }

    pub fn load_botdef_on_disk(path: &str) -> serde_json::Result<BotDef> {
        use std::fs::OpenOptions;
        use std::io::prelude::*;
        use std::io::{BufReader, BufWriter};
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        let mut buf_r = BufReader::new(file);
        serde_json::from_reader(buf_r)
    }

    pub fn load_botdef_in_editor(
        path: &str,
        unit_editor: &mut UnitEditor,
        unit_part_gpu: &mut unit_part_gpu::UnitPartGpu,
    ) {
        if let Ok(botdef) = Self::load_botdef_on_disk(path) {
            log::info!("Loaded {:?}", botdef.id);
            unit_editor.botdef = botdef;

            //Might need to clone
            for node in unit_editor.botdef.clone().part_tree.iter() {
                if let Some(mesh) = &node.placed_mesh {
                    let index = unit_part_gpu.index_of_or_create_if_na(mesh.mesh_path.clone());

                    //Update the mesh index, because it depend on the load order in the unit_part_gpu
                    for node_mut in unit_editor.botdef.part_tree.find_node_mut(node.id) {
                        for pm in &mut node_mut.placed_mesh {
                            pm.mesh_index = index;
                        }
                    }
                }
            }
        }
    }

    fn ui_part_tree(
        ui: &Ui,
        part_tree: &PartTree,
        unit_editor: &mut UnitEditor,
        is_root: bool,
        unit_part_gpu: &mut UnitPartGpu,
    ) {
        {
            if !is_root {
                ui.same_line(0.0);
                if ui.button(im_str!("remove##{:?}", part_tree.id).as_ref(), [0.0, 0.0]) {
                    let deleter = unit_editor.botdef.part_tree.remove_node(part_tree.id);
                }
            }

            let add_str = im_str!("Add child##{:?}", part_tree.id);
            if ui.button(add_str.as_ref(), [0.0, 0.0]) {
                ui.open_popup(add_str.as_ref());
            }
            ui.popup_modal(add_str.as_ref())
                .always_auto_resize(true)
                .build(|| {
                    Self::visit_dirs_for_add_child(
                        &unit_editor.asset_dir_cached.clone(),
                        ui,
                        unit_editor,
                        unit_part_gpu,
                        part_tree.id,
                    );

                    if ui.button(im_str!("Close"), [0.0, 0.0]) {
                        ui.close_current_popup();
                    }
                });
            ui.tree_node(im_str!("children").as_ref())
                .default_open(true)
                .build(|| {
                    for c in part_tree.children.iter() {
                        {
                            ui.tree_node(im_str!("child##{:?}", c.id).as_ref())
                                .default_open(true)
                                .build(|| {
                                    let name = im_str!("child");
                                    ChildWindow::new(name)
                                        .border(true)
                                        .always_auto_resize(true)
                                        .build(ui, || {
                                            let ui_for_transform =
                                            |id: String, matrix: Matrix4<f32>| -> Matrix4<f32> {
                                                let pos = matrix * Vector4::new(0.0, 0.0, 0.0, 1.0);
                                                let pos = pos.xyz() / pos.w;
                                                let arr_pos: &mut [f32; 3] =
                                                    &mut [pos.x, pos.y, pos.z];
                                                ui.drag_float3(
                                                    im_str!("position##{:?}", id).as_ref(),
                                                    arr_pos,
                                                )
                                                .speed(0.001)
                                                .min(-3.0)
                                                .max(3.0)
                                                .build();
                                                let isometry: Isometry3<f32> = unsafe {
                                                    na::convert_unchecked::<
                                                        Matrix4<f32>,
                                                        Isometry3<f32>,
                                                    >(
                                                        matrix
                                                    )
                                                };
                                                let euler = isometry.rotation.euler_angles();
                                                let arr_angle: &mut [f32; 3] =
                                                    &mut [euler.0, euler.1, euler.2];
                                                ui.drag_float3(
                                                    im_str!("euler angles##{:?}", id).as_ref(),
                                                    arr_angle,
                                                )
                                                .speed(0.001)
                                                .min(-6.0)
                                                .max(6.0)
                                                .build();
                                                let rotation_mat = Matrix4::from_euler_angles(
                                                    arr_angle[0],
                                                    arr_angle[1],
                                                    arr_angle[2],
                                                );
                                                let final_mat = utils::face_towards_dir(
                                                    &Vector3::new(
                                                        arr_pos[0], arr_pos[1], arr_pos[2],
                                                    ),
                                                    &Vector3::new(1.0, 0.0, 0.0),
                                                    &Vector3::new(0.0, 0.0, 1.0),
                                                ) * rotation_mat;

                                                final_mat
                                            };

                                            ui.text(im_str!("node transform:"));
                                            let new_parent_to_self = ui_for_transform(
                                                format!("{:?}", c.id),
                                                c.parent_to_self,
                                            );

                                            //Joint
                                            ui.text(im_str!("joint {:?}", c.joint));
                                            ui.same_line(0.0);
                                            if ui.small_button(im_str!("swap##{:?}", c.id).as_ref())
                                            {
                                                unit_editor
                                                    .botdef
                                                    .part_tree
                                                    .find_node_mut(c.id)
                                                    .unwrap()
                                                    .joint
                                                    .replace_with_next();
                                            }

                                            if let Some(model) = &c.placed_mesh {
                                                ui.text(im_str!("mesh: {:?}", model.mesh_path));
                                            }
                                            let replace_str = im_str!("replace mesh##{:?}", c.id);
                                            if ui.button(replace_str.as_ref(), [0.0, 0.0]) {
                                                ui.open_popup(replace_str.as_ref());
                                            }
                                            ui.popup_modal(replace_str.as_ref())
                                                .always_auto_resize(true)
                                                .build(|| {
                                                    Self::visit_dirs_for_replace_mesh(
                                                        &unit_editor.asset_dir_cached.clone(),
                                                        ui,
                                                        unit_editor,
                                                        unit_part_gpu,
                                                        c.id,
                                                    );
                                                    if ui.button(im_str!("Close"), [0.0, 0.0]) {
                                                        ui.close_current_popup();
                                                    }
                                                });

                                            ui.text(im_str!("mesh transform:"));
                                            let new_placed_mesh =
                                                if let Some(Some(old_placed_mesh)) = unit_editor
                                                    .botdef
                                                    .part_tree
                                                    .find_node(c.id)
                                                    .map(|e| &e.placed_mesh)
                                                {
                                                    let new_placed_mesh = PlacedMesh {
                                                        trans: ui_for_transform(
                                                            format!("placed_mesh{:?}", c.id),
                                                            old_placed_mesh.trans,
                                                        ),
                                                        mesh_path: old_placed_mesh
                                                            .mesh_path
                                                            .clone(),
                                                        mesh_index: old_placed_mesh.mesh_index,
                                                    };
                                                    Some(new_placed_mesh)
                                                } else {
                                                    None
                                                };

                                            if let Some(node) =
                                                unit_editor.botdef.part_tree.find_node_mut(c.id)
                                            {
                                                node.parent_to_self = new_parent_to_self;
                                                node.placed_mesh = new_placed_mesh;
                                            };

                                            Self::ui_part_tree(
                                                ui,
                                                &c,
                                                unit_editor,
                                                false,
                                                unit_part_gpu,
                                            );
                                        })
                                });
                        };
                    }
                });
        };
    }

    fn visit_dirs_for_add_child(
        dir: &FileTree,
        ui: &Ui,
        unit_editor: &mut UnitEditor,
        unit_part_gpu: &mut UnitPartGpu,
        parent: utils::Id<PartTree>,
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

                    let (index, state) = unit_part_gpu.path_get_or_create_if_na(path.to_owned());
                    match state {
                        ModelGpuState::Ready(_) | ModelGpuState::ToLoad(_) => {
                            if ui.small_button(im_str!("add to parts##{:?}", path).as_ref()) {
                                log::debug!("add to parts {:?}", path);
                                unit_editor.add_to_parts(parent, path.clone(), index);
                            }
                            ui.same_line(0.0);
                            if ui.small_button(im_str!("reload##{:?}", path).as_ref()) {
                                unit_part_gpu.reload(path.clone());
                            }
                        }
                        ModelGpuState::Error(e) => {
                            ui.text_colored([1.0, 0.0, 0.0, 1.0], im_str!("Error"));
                            ui.same_line(0.0);
                            if ui.small_button(im_str!("reload##{:?}", path).as_ref()) {
                                unit_part_gpu.reload(path.clone());
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
                        Self::visit_dirs_for_add_child(
                            &child,
                            ui,
                            unit_editor,
                            unit_part_gpu,
                            parent,
                        );
                    }
                });
            }
        }
    }

    fn visit_dirs_for_replace_mesh(
        dir: &FileTree,
        ui: &Ui,
        unit_editor: &mut UnitEditor,
        unit_part_gpu: &mut UnitPartGpu,
        id_to_mesh_replace: utils::Id<PartTree>,
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

                    let mut replace_exe = |mesh_index| {
                        if let Some(child) = unit_editor
                            .botdef
                            .part_tree
                            .find_node_mut(id_to_mesh_replace)
                        {
                            if let Some(old) = &child.placed_mesh {
                                child.placed_mesh = Some(PlacedMesh {
                                    mesh_index,
                                    mesh_path: path.clone(),
                                    trans: old.trans.clone(),
                                });
                            } else {
                                child.placed_mesh = Some(PlacedMesh {
                                    mesh_index,
                                    mesh_path: path.clone(),
                                    trans: Matrix4::identity(),
                                });
                            }
                        }
                    };

                    let state = unit_part_gpu.path_get(path.to_owned());
                    match state {
                        None => {
                            if ui.small_button(im_str!("replace with this##{:?}", path).as_ref()) {
                                log::debug!("replace with this {:?}", path);
                                log::debug!("was not open {:?}", path);

                                let index = unit_part_gpu.index_of_or_create_if_na(path.to_owned());
                                replace_exe(index);
                            }
                        }
                        Some(ModelGpuState::Ready(_)) | Some(ModelGpuState::ToLoad(_)) => {
                            if ui.small_button(im_str!("replace with this##{:?}", path).as_ref()) {
                                log::debug!("replace with this {:?}", path);
                                let index = unit_part_gpu.index_of_or_create_if_na(path.to_owned());
                                replace_exe(index);
                            }
                            ui.same_line(0.0);
                            if ui.small_button(im_str!("reload##{:?}", path).as_ref()) {
                                unit_part_gpu.reload(path.clone());
                            }
                        }
                        Some(ModelGpuState::Error(e)) => {
                            ui.text_colored([1.0, 0.0, 0.0, 1.0], im_str!("Error"));
                            ui.same_line(0.0);
                            if ui.small_button(im_str!("reload##{:?}", path).as_ref()) {
                                unit_part_gpu.reload(path.clone());
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
                        Self::visit_dirs_for_replace_mesh(
                            &child,
                            ui,
                            unit_editor,
                            unit_part_gpu,
                            id_to_mesh_replace,
                        );
                    }
                });
            }
        }
    }
}

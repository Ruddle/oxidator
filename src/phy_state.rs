use na::Vector3;
use nphysics3d::force_generator::DefaultForceGeneratorSet;
use nphysics3d::joint::DefaultJointConstraintSet;
use nphysics3d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodyHandle, DefaultBodySet, DefaultColliderHandle,
    DefaultColliderSet,
};
use nphysics3d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};

use crossbeam_channel::{Receiver, Sender};
use na::{Isometry3, Matrix3, Matrix4, Point3};
use nalgebra::DMatrix;
use ncollide3d::shape::{Ball, HeightField, ShapeHandle};
use nphysics3d::math::{Inertia, Velocity};
use nphysics3d::object::{BodyStatus, Ground, RigidBodyDesc};

pub struct InnerState {
    mechanical_world: DefaultMechanicalWorld<f32>,
    geometrical_world: DefaultGeometricalWorld<f32>,
    bodies: DefaultBodySet<f32>,
    colliders: DefaultColliderSet<f32>,
    joint_constraints: DefaultJointConstraintSet<f32>,
    force_generators: DefaultForceGeneratorSet<f32>,
    cubes: Vec<(DefaultBodyHandle, DefaultColliderHandle)>,

    s_outer: Sender<ToOuter>,
    r_inner: Receiver<ToInner>,
}

pub struct State {
    balls: Vec<Matrix4<f32>>,
    perf: String,

    s_inner: Sender<ToInner>,
    r_outer: Receiver<ToOuter>,
}

pub enum ToInner {
    Step,
}

pub enum ToOuter {
    Isos(Vec<Isometry3<f32>>),
    Perf(String),
}

impl State {
    pub fn new() -> Self {
        let (s_inner, r_inner) = crossbeam_channel::bounded::<ToInner>(1);
        let (s_outer, r_outer) = crossbeam_channel::unbounded::<ToOuter>();

        std::thread::spawn(move || {
            let inner = InnerState::new(r_inner, s_outer);
            inner.do_loop();
        });

        State {
            balls: Vec::new(),
            perf: "".to_string(),
            s_inner,
            r_outer,
        }
    }

    pub fn step(&mut self) {
        for msg in self.r_outer.try_iter() {
            match msg {
                ToOuter::Perf(s) => {
                    self.perf = s;
                }
                ToOuter::Isos(isos) => {
                    let corr: Matrix4<f32> = Matrix4::new(
                        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    );

                    self.balls = isos.iter().map(|iso| corr * iso.to_homogeneous()).collect();
                }
            }
        }

        let _ = self.s_inner.try_send(ToInner::Step);
    }

    pub fn draw_ui(&self, ui: &imgui::Ui) {
        use imgui::*;

        let edit_height_window = imgui::Window::new(im_str!("Physic statistics"));
        edit_height_window
            .size([400.0, 230.0], imgui::Condition::FirstUseEver)
            .position([3.0, 412.0], imgui::Condition::FirstUseEver)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text(im_str!("{}", self.perf));
            });
    }

    pub fn cubes_transform(&self) -> &Vec<Matrix4<f32>> {
        &self.balls
    }
}

impl InnerState {
    pub fn new(r_inner: Receiver<ToInner>, s_outer: Sender<ToOuter>) -> Self {
        let mut mechanical_world = DefaultMechanicalWorld::new(Vector3::new(0.0, -10.0, 0.0));
        let mut geometrical_world = DefaultGeometricalWorld::new();

        let mut bodies = DefaultBodySet::new();
        let mut colliders = DefaultColliderSet::new();
        let mut joint_constraints = DefaultJointConstraintSet::new();
        let mut force_generators = DefaultForceGeneratorSet::new();

        mechanical_world.set_timestep(1.0 / 60.0);
        mechanical_world.counters.enable();
        mechanical_world
            .integration_parameters
            .multiple_ccd_substep_sensor_events_enabled = false;
        mechanical_world
            .integration_parameters
            .max_position_iterations = 1;
        mechanical_world
            .integration_parameters
            .max_velocity_iterations = 1;

        mechanical_world
            .integration_parameters
            .max_ccd_position_iterations = 1;

        mechanical_world.integration_parameters.allowed_linear_error = 0.01;
        mechanical_world
            .integration_parameters
            .allowed_angular_error = 0.01;
        let step = 2048;

        let ground_handle = bodies.insert(Ground::new());
        for i in (0..2048 / step).step_by(1) {
            for j in (0..2048 / step).step_by(1) {
                let heights = DMatrix::from_fn(step, step, |a, b| {
                    crate::heightmap::z((i * step) as f32 + b as f32, (j * step) as f32 + a as f32)
                });
                let heightfield: HeightField<f32> =
                    HeightField::new(heights, Vector3::new(step as f32, 1.0, step as f32));

                //                let shape =
                //                    ShapeHandle::new(ncollide3d::shape::Plane::new(Vector3::<f32>::y_axis()));
                //
                //                let quad = ncollide3d::procedural::quad(2048.0, 2048.0, 1, 1);
                //                let indices = quad
                //                    .flat_indices()
                //                    .chunks(3)
                //                    .map(|is| Point3::new(is[0] as usize, is[2] as usize, is[1] as usize))
                //                    .collect();
                //
                //                let mut vertices = quad.coords;
                //
                //                for p in &mut vertices {
                //                    p.x = p.x + 1024.0;
                //                    p.z = p.y + 1024.0;
                //                    p.y = crate::heightmap::z(p.x, p.z);
                //                }
                //
                //                let shape =
                //                    ShapeHandle::new(ncollide3d::shape::TriMesh::new(vertices, indices, None));

                let shape = ShapeHandle::new(heightfield);
                //    let shape = ShapeHandle::new(Ball::new(1.0));
                let ground_col = ColliderDesc::new(shape)
                    .position(Isometry3::translation(1024.0, 0.0, 1024.0))
                    .build(BodyPartHandle(ground_handle, 0));
                let ground_col_handle = colliders.insert(ground_col);
            }
        }

        let mut cubes = Vec::new();
        for i in (100..2000).step_by(40) {
            for j in (100..2000).step_by(40) {
                let ball_rb = RigidBodyDesc::new()
                    .translation(Vector3::y() * 5.0)
                    .rotation(Vector3::y() * 5.0)
                    .position(Isometry3::new(
                        Vector3::new(i as f32, 200.0, j as f32),
                        Vector3::y() * 3.141592,
                    ))
                    .gravity_enabled(true)
                    .sleep_threshold(None)
                    .build();

                let ball_rb_handle = bodies.insert(ball_rb);
                let shape =
                    ShapeHandle::new(ncollide3d::shape::Cuboid::new(Vector3::new(0.5, 0.5, 0.5)));
                let ball_col = ColliderDesc::new(shape)
                    // The collider density. If non-zero the collider's mass and angular inertia will be added
                    // to the inertial properties of the body part it is attached to.
                    // Default: 0.0
                    .density(1.3)
                    //                    .ccd_enabled(true)
                    .build(BodyPartHandle(ball_rb_handle, 0));

                let ball_col_handle = colliders.insert(ball_col);

                cubes.push((ball_rb_handle, ball_col_handle));
            }
        }

        println!("Number of cubes {}", cubes.len());

        InnerState {
            mechanical_world,
            geometrical_world,
            bodies,
            colliders,
            joint_constraints,
            force_generators,
            cubes,

            r_inner,
            s_outer,
        }
    }

    pub fn do_loop(mut self) {
        loop {
            match self.r_inner.recv().unwrap() {
                Step => {
                    self.step();
                    let _ = self.s_outer.try_send(ToOuter::Perf(self.get_perf_string()));
                    let _ = self.s_outer.try_send(ToOuter::Isos(self.balls_isos()));
                }
            }
        }
    }

    pub fn step(&mut self) {
        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        );
    }

    pub fn get_perf_string(&self) -> String {
        let counters = self.mechanical_world.counters;
        let profile = format!(
            r#"Total: {:.2}ms
Collision detection: {:.2}ms
|_ Broad-phase: {:.2}ms
   Narrow-phase: {:.2}ms
Island computation: {:.2}ms
Solver: {:.2}ms
|_ Assembly: {:.2}ms
   Velocity resolution: {:.2}ms
   Position resolution: {:.2}ms
CCD: {:.2}ms
|_ # of substeps: {}
   TOI computation: {:.2}ms
   Broad-phase: {:.2}ms
   Narrow-phase: {:.2}ms
   Solver: {:.2}ms"#,
            counters.step_time() * 1000.0,
            counters.collision_detection_time() * 1000.0,
            counters.broad_phase_time() * 1000.0,
            counters.narrow_phase_time() * 1000.0,
            counters.island_construction_time() * 1000.0,
            counters.solver_time() * 1000.0,
            counters.assembly_time() * 1000.0,
            counters.velocity_resolution_time() * 1000.0,
            counters.position_resolution_time() * 1000.0,
            counters.ccd_time() * 1000.0,
            counters.ccd.num_substeps,
            counters.ccd.toi_computation_time.time() * 1000.0,
            counters.ccd.broad_phase_time.time() * 1000.0,
            counters.ccd.narrow_phase_time.time() * 1000.0,
            counters.ccd.solver_time.time() * 1000.0
        );
        profile
    }

    pub fn balls_isos(&self) -> Vec<Isometry3<f32>> {
        self.cubes
            .iter()
            .map(|(bh, ch)| {
                let rigid_body = self.bodies.rigid_body(*bh).unwrap();
                *rigid_body.position()
            })
            .collect()
    }
}

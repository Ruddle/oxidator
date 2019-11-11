use super::client::*;
use crate::frame::FrameEventFromPlayer;
use crate::frame::Player;
use crate::*;
use imgui::*;
use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use utils::*;

impl App {
    pub fn init_play(&mut self) {
        match self.net_mode {
            NetMode::Offline | NetMode::Server => {
                self.clear_gpu_instance_and_game_state();
                self.game_state.position =
                    Point3::new(300.0, 100.0, self.heightmap_gpu.phy.z(300.0, 100.0) + 50.0);
                self.game_state.dir = Vector3::new(0.0, 0.3, -1.0);

                let mut player_me = Player::new();

                let mut kbots = HashMap::new();

                let tank_example = unit::PartTree {
                    id: utils::rand_id(),
                    dmodel: None,
                    joint: unit::Joint::Fix,
                    children: vec![unit::PartTree {
                        id: utils::rand_id(),
                        dmodel: Some(unit::DisplayModel {
                            //Z is Y ?
                            position: Point3::new(0.0, 0.0, 0.0),
                            dir: Vector3::new(1.0, 0.0, 0.0),
                            model_path: Path::new("./src/asset/cube.obj").to_owned(),
                        }),
                        joint: unit::Joint::Fix,
                        children: vec![],
                    }],
                };

                for i in (100..300).step_by(4) {
                    for j in (100..500).step_by(4) {
                        let m = mobile::KBot::new(
                            Point3::new(i as f32, j as f32, 100.0),
                            tank_example.id,
                        );
                        player_me.kbots.insert(m.id);
                        kbots.insert(m.id, m);
                    }
                }

                let mut player_ennemy = Player::new();
                player_ennemy.team = 1;

                for i in (320..520).step_by(4) {
                    for j in (100..500).step_by(4) {
                        let mut m = mobile::KBot::new(
                            Point3::new(i as f32, j as f32, 100.0),
                            tank_example.id,
                        );
                        m.team = 1;
                        player_ennemy.kbots.insert(m.id);
                        kbots.insert(m.id, m);
                    }
                }

                log::info!("Starting a game with {} bots", kbots.len());

                self.game_state.my_player_id = Some(player_me.id);
                self.game_state.players.insert(player_me.id, player_me);
                self.game_state
                    .players
                    .insert(player_ennemy.id, player_ennemy);

                let mut part_trees = HashMap::new();
                part_trees.insert(tank_example.id, tank_example);

                let replacer = FrameEventFromPlayer::ReplaceFrame(frame::Frame {
                    number: 0,
                    players: self.game_state.players.clone(),
                    kbots,
                    kbots_dead: HashSet::new(),
                    kinematic_projectiles_dead: Vec::new(),
                    kinematic_projectiles_birth: Vec::new(),
                    kinematic_projectiles: self.game_state.kinematic_projectiles_cache.clone(),
                    arrows: Vec::new(),
                    explosions: Vec::new(),
                    heightmap_phy: Some(self.heightmap_gpu.phy.clone()),
                    frame_profiler: frame::ProfilerMap::new(),
                    part_trees,
                });
                let _ = self
                    .sender_from_client_to_manager
                    .try_send(client::FromClient::PlayerInput(replacer));
            }

            NetMode::Client => {
                self.clear_gpu_instance_and_game_state();
                self.game_state.position =
                    Point3::new(300.0, 100.0, self.heightmap_gpu.phy.z(300.0, 100.0) + 50.0);
                self.game_state.dir = Vector3::new(0.0, 0.3, -1.0);
                self.game_state.my_player_id = self
                    .game_state
                    .frame_zero
                    .players
                    .values()
                    .filter(|p| p.team == 1)
                    .map(|p| p.id.clone())
                    .next();
            }
        }
    }

    pub fn handle_play(
        &mut self,
        delta_sim_sec: f32,
        encoder: &mut wgpu::CommandEncoder,
        view_proj: &Matrix4<f32>,
    ) {
        //Interpolate
        let interp_duration = time(|| {
            self.game_state.interpolate(&self.threadpool, &view_proj);
        });

        // Selection on screen
        let selection_screen = time(|| {
            if let Some(me) = self.game_state.my_player() {
                //Selection square
                if let input_state::Drag::End { x0, y0, x1, y1 } = self.input_state.drag {
                    let start_sel = std::time::Instant::now();
                    let min_x = (x0.min(x1) as f32 / self.gpu.sc_desc.width as f32) * 2.0 - 1.0;
                    let min_y = (y0.min(y1) as f32 / self.gpu.sc_desc.height as f32) * 2.0 - 1.0;
                    let max_x = (x0.max(x1) as f32 / self.gpu.sc_desc.width as f32) * 2.0 - 1.0;
                    let max_y = (y0.max(y1) as f32 / self.gpu.sc_desc.height as f32) * 2.0 - 1.0;
                    let selected: HashSet<IdValue> = self
                        .game_state
                        .kbots
                        .iter()
                        .filter(|e| {
                            e.is_in_screen
                                && me.kbots.contains(&e.id)
                                && e.screen_pos.x > min_x
                                && e.screen_pos.x < max_x
                                && e.screen_pos.y < max_y
                                && e.screen_pos.y > min_y
                        })
                        .map(|e| e.id.value)
                        .collect();

                    log::trace!("Selection took {}us", start_sel.elapsed().as_micros());

                    self.game_state.selected = selected;
                } else if self
                    .input_state
                    .mouse_release
                    .contains(&winit::event::MouseButton::Left)
                {
                    self.game_state.selected.clear();
                }
            }
        });

        self.profiler.mix("interp", interp_duration, 20);
        self.profiler.mix("selection_screen", selection_screen, 20);
    }
}

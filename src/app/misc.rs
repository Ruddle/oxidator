use super::app::*;
use crate::*;
use imgui::*;
use na::{Isometry3, Matrix4, Point3, Vector2, Vector3, Vector4};
use std::collections::{HashMap, HashSet};

use utils::time;

impl App {
    pub fn clear_from_play(&mut self) {
        self.game_state.mobiles.clear();
        self.game_state.selected.clear();
        self.mobile_gpu.update_instance(&[], &self.gpu.device);
    }
}

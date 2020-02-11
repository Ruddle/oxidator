use crate::unit;
use crate::utils;
use serde::{Deserialize, Serialize};
use typename::TypeName;
use utils::Id;

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct BotDef {
    pub id: Id<BotDef>,
    pub radius: f32,
    pub max_life: i32,
    //Movement
    ///rad/frame²
    pub turn_accel: f32,
    ///rad/frame
    pub max_turn_rate: f32,
    ///m/frame²
    pub accel: f32,
    ///m/frame²
    pub break_accel: f32,
    ///m/frame
    pub max_speed: f32,
    ///metal/frame
    pub build_power: i32,
    ///m
    pub build_dist: f32,
    ///metal
    pub metal_cost: i32,

    pub part_tree: unit::PartTree,
}

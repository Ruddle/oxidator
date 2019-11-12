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
    pub part_tree: unit::PartTree,
}

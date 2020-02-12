use crate::botdef;
use crate::*;
use utils::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiTool {
    None,
    Move,
    Repair,
    Guard,
    Attack,
    Spawn(Id<botdef::BotDef>),
}

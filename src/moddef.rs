use crate::botdef::BotDef;
use crate::unit;
use crate::utils;
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use typename::TypeName;
use utils::Id;

#[derive(Clone, TypeName, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModDef {
    pub units_id: Vec<Id<BotDef>>,
    pub con_map: FnvHashMap<Id<BotDef>, Vec<Id<BotDef>>>,
}

impl ModDef {
    pub fn new() -> Self {
        Self {
            units_id: Vec::new(),
            con_map: FnvHashMap::default(),
        }
    }
}

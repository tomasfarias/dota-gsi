use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum BuildingsError {
    #[error("attempted to parse an empty building")]
    EmptyBuilding,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuildingInformation {
    health: u32,
    max_health: u32,
}

pub enum BuildingClass {
    Rax,
    Ancient,
    Tower,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Buildings {
    #[serde(flatten)]
    inner: HashMap<String, BuildingInformation>,
}

impl Buildings {
    pub fn get_building_information(&self, name: &str) -> Option<&BuildingInformation> {
        match self.inner.get(name) {
            Some(i) => Some(i),
            None => None,
        }
    }

    pub fn contains_building(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buildings_deserialize() {
        let json_str = r#"{
    "bad_rax_melee_bot": {
      "health": 2200,
      "max_health": 2200
    },
    "bad_rax_melee_mid": {
      "health": 2200,
      "max_health": 2200
    },
    "bad_rax_melee_top": {
      "health": 2200,
      "max_health": 2200
    },
    "bad_rax_range_bot": {
      "health": 1300,
      "max_health": 1300
    },
    "bad_rax_range_mid": {
      "health": 1300,
      "max_health": 1300
    },
    "bad_rax_range_top": {
      "health": 1300,
      "max_health": 1300
    },
    "dota_badguys_fort": {
      "health": 4500,
      "max_health": 4500
    },
    "dota_badguys_tower1_bot": {
      "health": 1752,
      "max_health": 1800
    },
    "dota_badguys_tower2_bot": {
      "health": 2500,
      "max_health": 2500
    },
    "dota_badguys_tower2_mid": {
      "health": 2395,
      "max_health": 2500
    },
    "dota_badguys_tower2_top": {
      "health": 2282,
      "max_health": 2500
    },
    "dota_badguys_tower3_bot": {
      "health": 2500,
      "max_health": 2500
    },
    "dota_badguys_tower3_mid": {
      "health": 2500,
      "max_health": 2500
    },
    "dota_badguys_tower3_top": {
      "health": 2500,
      "max_health": 2500
    },
    "dota_badguys_tower4_bot": {
      "health": 2600,
      "max_health": 2600
    },
    "dota_badguys_tower4_top": {
      "health": 2600,
      "max_health": 2600
    }
  }"#;
        let buildings: Buildings =
            serde_json::from_str(json_str).expect("Failed to deserialize Buildings");

        assert!(buildings.contains_building("dota_badguys_tower3_mid"));
    }
}

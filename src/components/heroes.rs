use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{PlayerID, Team};

#[derive(Serialize, Deserialize, Debug)]
pub struct Hero {
    pub xpos: Option<i32>,
    pub ypos: Option<i32>,
    pub id: i16,
    pub name: Option<String>,
    pub level: Option<u8>,
    pub xp: Option<u32>,
    pub alive: Option<bool>,
    pub respawn_seconds: Option<u16>,
    pub buyback_cost: Option<u16>,
    pub buyback_cooldown: Option<u16>,
    pub health: Option<u16>,
    pub max_health: Option<u16>,
    pub health_percent: Option<u8>,
    pub mana: Option<u16>,
    pub max_mana: Option<u16>,
    pub mana_percent: Option<u16>,
    pub silenced: Option<bool>,
    pub stunned: Option<bool>,
    pub disarmed: Option<bool>,
    pub magicimmune: Option<bool>,
    pub hexed: Option<bool>,
    pub muted: Option<bool>,
    pub r#break: Option<bool>,
    pub aghanims_scepter: Option<bool>,
    pub aghanims_shard: Option<bool>,
    pub smoked: Option<bool>,
    pub has_debuff: Option<bool>,
    pub talent_1: Option<bool>,
    pub talent_2: Option<bool>,
    pub talent_3: Option<bool>,
    pub talent_4: Option<bool>,
    pub talent_5: Option<bool>,
    pub talent_6: Option<bool>,
    pub talent_7: Option<bool>,
    pub talent_8: Option<bool>,
}

impl fmt::Display for Hero {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.name {
            None => {
                write!(f, "No Hero")
            }
            Some(name) => {
                write!(f, "Hero {}", name)
            }
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum GameHeroes {
    Spectating(HashMap<Team, HashMap<PlayerID, Hero>>),
    Playing(Hero),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hero_selection() {
        let json_str = r#"{
        "id": -1
      }"#;

        let hero: Hero = serde_json::from_str(json_str).expect("Failed to deserialize Hero");

        assert_eq!(hero.id, -1);
        assert_eq!(hero.name, None);
    }

    #[test]
    fn test_hero_deserialize() {
        let json_str = r#"{
        "aghanims_scepter": false,
        "aghanims_shard": false,
        "alive": true,
        "break": false,
        "buyback_cooldown": 0,
        "buyback_cost": 379,
        "disarmed": false,
        "has_debuff": false,
        "health": 1045,
        "health_percent": 95,
        "hexed": false,
        "id": 136,
        "level": 7,
        "magicimmune": false,
        "mana": 721,
        "mana_percent": 100,
        "max_health": 1100,
        "max_mana": 721,
        "muted": false,
        "name": "npc_dota_hero_marci",
        "respawn_seconds": 0,
        "selected_unit": true,
        "silenced": false,
        "smoked": false,
        "stunned": false,
        "talent_1": false,
        "talent_2": false,
        "talent_3": false,
        "talent_4": false,
        "talent_5": false,
        "talent_6": false,
        "talent_7": false,
        "talent_8": false,
        "xp": 3238,
        "xpos": -4267,
        "ypos": 2310
      }"#;

        let hero: Hero = serde_json::from_str(json_str).expect("Failed to deserialize Hero");

        assert_eq!(hero.name, Some(String::from("npc_dota_hero_marci")));
        assert_eq!(hero.max_health, Some(1100));
    }
}

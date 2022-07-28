use std::collections::HashMap;
use std::fmt;

use serde::{de, de::Error, ser, Deserialize, Serialize};
use thiserror;

use super::{PlayerID, Team};

#[derive(thiserror::Error, Debug)]
pub enum AbilitiesError {
    #[error("failed to parse ability ID number in `{0}`")]
    ParseIDError(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ability {
    name: String,
    level: u8,
    can_cast: bool,
    passive: bool,
    ability_active: bool,
    cooldown: u16,
    ultimate: bool,
}

impl fmt::Display for Ability {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut cd_status = String::from("");

        if self.can_cast && !self.passive {
            cd_status.push_str("READY");
        } else if self.passive {
            cd_status.push_str("PASSIVE");
        } else {
            let cd_str = format!("IN CD: {}s", self.cooldown);
            cd_status.push_str(&cd_str);
        }

        write!(f, "{} level {}, {}", self.name, self.level, cd_status)
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct AbilityID(u8);

impl<'de> Deserialize<'de> for AbilityID {
    fn deserialize<D>(deserializer: D) -> Result<AbilityID, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut slot_split = s.split("ability").map(|s| s.parse::<u8>());

        if let (_, Some(index)) = (slot_split.next(), slot_split.next()) {
            return Ok(AbilityID(index.expect("failed to parse ID")));
        }

        Err(AbilitiesError::ParseIDError(s)).map_err(D::Error::custom)
    }
}

impl Serialize for AbilityID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&format!("ability{}", self.0))
    }
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum GameAbilities {
    Spectating(HashMap<Team, HashMap<PlayerID, HashMap<AbilityID, Ability>>>),
    Playing(HashMap<AbilityID, Ability>),
    NotInGame {},
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abilities_deserialize() {
        let json_str = r#"[{
          "ability_active": true,
          "can_cast": true,
          "cooldown": 0,
          "level": 4,
          "name": "marci_grapple",
          "passive": false,
          "ultimate": false
        },
        {
          "ability_active": true,
          "can_cast": true,
          "cooldown": 0,
          "level": 1,
          "name": "marci_companion_run",
          "passive": false,
          "ultimate": false
        },
        {
          "ability_active": true,
          "can_cast": true,
          "cooldown": 0,
          "level": 1,
          "name": "marci_guardian",
          "passive": false,
          "ultimate": false
        },
        {
          "ability_active": true,
          "can_cast": true,
          "cooldown": 0,
          "level": 1,
          "name": "marci_unleash",
          "passive": false,
          "ultimate": true
        },
        {
          "ability_active": true,
          "can_cast": true,
          "cooldown": 0,
          "level": 1,
          "name": "plus_high_five",
          "passive": false,
          "ultimate": false
        },
        {
          "ability_active": true,
          "can_cast": true,
          "cooldown": 0,
          "level": 1,
          "name": "plus_guild_banner",
          "passive": false,
          "ultimate": false
        }
      ]"#;
        let abilities: Vec<Ability> =
            serde_json::from_str(json_str).expect("Failed to deserialize Abilities");
    }
}

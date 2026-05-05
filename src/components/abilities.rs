use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize, de, de::Error, ser};
use thiserror;

use super::{PlayerID, Team};

#[cfg(feature = "diff")]
use crate::diff::Diffable;
#[cfg(feature = "diff")]
use crate::event::{Ability as AbilityEvent, GameEvent};

#[derive(thiserror::Error, Debug)]
pub enum AbilitiesError {
    #[error("failed to parse ability ID number in `{0}`")]
    ParseIDError(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Ability {
    pub name: String,
    pub level: u8,
    pub can_cast: bool,
    pub passive: bool,
    pub ability_active: bool,
    pub cooldown: u16,
    pub ultimate: bool,
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

#[cfg(feature = "diff")]
impl Diffable for Ability {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if self.level < new.level {
            events.push(GameEvent::AbilityEvent(AbilityEvent::LevelledUp {
                name: self.name.clone(),
                level: new.level,
                ultimate: self.ultimate,
            }));
        }

        match (self.can_cast, new.can_cast) {
            (true, false) => events.push(GameEvent::AbilityEvent(AbilityEvent::WentOnCooldown {
                name: self.name.clone(),
                remaining: new.cooldown,
                ultimate: self.ultimate,
            })),
            (false, true) => events.push(GameEvent::AbilityEvent(AbilityEvent::WentOffCooldown {
                name: self.name.clone(),
                ultimate: self.ultimate,
            })),
            _ => {}
        }

        match (self.ability_active, new.ability_active) {
            (true, false) => events.push(GameEvent::AbilityEvent(AbilityEvent::Deactivated {
                name: self.name.clone(),
                ultimate: self.ultimate,
            })),
            (false, true) => events.push(GameEvent::AbilityEvent(AbilityEvent::Activated {
                name: self.name.clone(),
                ultimate: self.ultimate,
            })),
            _ => {}
        }

        events
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct AbilityID(pub u8);

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

        Err(D::Error::custom(AbilitiesError::ParseIDError(s)))
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

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum GameAbilities {
    Spectating(HashMap<Team, HashMap<PlayerID, HashMap<AbilityID, Ability>>>),
    Playing(HashMap<AbilityID, Ability>),
}

#[cfg(feature = "diff")]
impl Diffable for GameAbilities {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        match (self, new) {
            (GameAbilities::Spectating(current), GameAbilities::Spectating(new)) => {
                for (team, players) in current.iter() {
                    let Some(team_new) = new.get(team) else {
                        continue;
                    };

                    for (player_id, abilities) in players.iter() {
                        let Some(abilities_new) = team_new.get(player_id) else {
                            continue;
                        };

                        for (ability_id, ability) in abilities.iter() {
                            let Some(ability_new) = abilities_new.get(ability_id) else {
                                continue;
                            };

                            events.extend(ability.diff(ability_new));
                        }
                    }
                }
            }
            (GameAbilities::Playing(abilities), GameAbilities::Playing(abilities_new)) => {
                for (ability_id, ability) in abilities.iter() {
                    let Some(ability_new) = abilities_new.get(ability_id) else {
                        continue;
                    };
                    events.extend(ability.diff(ability_new));
                }
            }
            (_, _) => panic!("cannot mix playing and spectating state"),
        }

        events
    }
}

#[cfg(test)]
pub(crate) mod tests {
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

        assert_eq!(abilities.len(), 6);
        assert!(abilities.iter().all(|a| a.ability_active));
        assert!(abilities.iter().all(|a| a.can_cast));
        assert!(
            abilities
                .iter()
                .any(|a| a.name == "plus_guild_banner".to_owned())
        );
        assert!(
            abilities
                .iter()
                .any(|a| a.name == "marci_unleash".to_owned())
        );
    }

    pub(crate) fn make_ability(level: u8, can_cast: bool, cooldown: u16, active: bool) -> Ability {
        Ability {
            name: "test_ability".to_string(),
            level,
            can_cast,
            passive: false,
            ability_active: active,
            cooldown,
            ultimate: false,
        }
    }

    #[test]
    fn test_ability_no_change() {
        let ability = make_ability(1, true, 0, true);
        let events = ability.diff(&ability.clone());
        assert!(events.is_empty());
    }

    #[test]
    fn test_ability_level_up() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(2, true, 0, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp {
                name: "test_ability".to_string(),
                level: 2,
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_ability_level_up_multiple() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(4, true, 0, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp {
                name: "test_ability".to_string(),
                level: 4,
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_ability_went_on_cooldown() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(1, false, 12, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::WentOnCooldown {
                name: "test_ability".to_string(),
                remaining: 12,
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_ability_went_off_cooldown() {
        let prev = make_ability(1, false, 5, true);
        let cur = make_ability(1, true, 0, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::WentOffCooldown {
                name: "test_ability".to_string(),
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_ability_still_on_cooldown_no_event() {
        let prev = make_ability(1, false, 10, true);
        let cur = make_ability(1, false, 5, true);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_ability_activated() {
        let prev = make_ability(1, true, 0, false);
        let cur = make_ability(1, true, 0, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::Activated {
                name: "test_ability".to_string(),
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_ability_deactivated() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(1, true, 0, false);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::Deactivated {
                name: "test_ability".to_string(),
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_ability_multiple_events_simultaneous() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(2, false, 8, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![
                GameEvent::AbilityEvent(AbilityEvent::LevelledUp {
                    name: "test_ability".to_string(),
                    level: 2,
                    ultimate: false
                }),
                GameEvent::AbilityEvent(AbilityEvent::WentOnCooldown {
                    name: "test_ability".to_string(),
                    remaining: 8,
                    ultimate: false
                }),
            ]
        );
    }

    #[test]
    fn test_game_abilities_playing_no_change() {
        let mut abilities = HashMap::new();
        abilities.insert(AbilityID(0), make_ability(1, true, 0, true));
        abilities.insert(AbilityID(1), make_ability(2, false, 5, true));

        let prev = GameAbilities::Playing(abilities.clone());
        let cur = GameAbilities::Playing(abilities);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_game_abilities_playing_one_levels_up() {
        let mut prev_abilities = HashMap::new();
        prev_abilities.insert(AbilityID(0), make_ability(1, true, 0, true));
        prev_abilities.insert(AbilityID(1), make_ability(2, true, 0, true));

        let mut cur_abilities = HashMap::new();
        cur_abilities.insert(AbilityID(0), make_ability(2, true, 0, true));
        cur_abilities.insert(AbilityID(1), make_ability(2, true, 0, true));

        let prev = GameAbilities::Playing(prev_abilities);
        let cur = GameAbilities::Playing(cur_abilities);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp {
                name: "test_ability".to_string(),
                level: 2,
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_game_abilities_playing_new_ability_ignored() {
        let mut prev_abilities = HashMap::new();
        prev_abilities.insert(AbilityID(0), make_ability(1, true, 0, true));

        let mut cur_abilities = HashMap::new();
        cur_abilities.insert(AbilityID(0), make_ability(1, true, 0, true));
        cur_abilities.insert(AbilityID(1), make_ability(1, true, 0, true));

        let prev = GameAbilities::Playing(prev_abilities);
        let cur = GameAbilities::Playing(cur_abilities);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_game_abilities_spectating_one_player_levels_up() {
        let mut radiant_prev = HashMap::new();
        let mut player0_abilities = HashMap::new();
        player0_abilities.insert(AbilityID(0), make_ability(1, true, 0, true));
        radiant_prev.insert(PlayerID(0), player0_abilities);

        let mut radiant_cur = HashMap::new();
        let mut player0_abilities_new = HashMap::new();
        player0_abilities_new.insert(AbilityID(0), make_ability(2, true, 0, true));
        radiant_cur.insert(PlayerID(0), player0_abilities_new);

        let mut prev_map = HashMap::new();
        prev_map.insert(Team::Radiant, radiant_prev);
        let mut cur_map = HashMap::new();
        cur_map.insert(Team::Radiant, radiant_cur);

        let prev = GameAbilities::Spectating(prev_map);
        let cur = GameAbilities::Spectating(cur_map);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp {
                name: "test_ability".to_string(),
                level: 2,
                ultimate: false
            })]
        );
    }

    #[test]
    fn test_game_abilities_spectating_missing_team_no_panic() {
        let mut prev_map = HashMap::new();
        let mut radiant = HashMap::new();
        let mut abilities_map = HashMap::new();
        abilities_map.insert(AbilityID(0), make_ability(1, true, 0, true));
        radiant.insert(PlayerID(0), abilities_map);
        prev_map.insert(Team::Radiant, radiant);

        // New state has no Radiant team
        let cur_map = HashMap::new();

        let prev = GameAbilities::Spectating(prev_map);
        let cur = GameAbilities::Spectating(cur_map);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_game_abilities_spectating_missing_player_no_panic() {
        let mut radiant_prev = HashMap::new();
        let mut p0_abilities = HashMap::new();
        p0_abilities.insert(AbilityID(0), make_ability(1, true, 0, true));
        radiant_prev.insert(PlayerID(0), p0_abilities);

        // New state has the team but not the player
        let radiant_cur: HashMap<PlayerID, HashMap<AbilityID, Ability>> = HashMap::new();

        let mut prev_map = HashMap::new();
        prev_map.insert(Team::Radiant, radiant_prev);
        let mut cur_map = HashMap::new();
        cur_map.insert(Team::Radiant, radiant_cur);

        let prev = GameAbilities::Spectating(prev_map);
        let cur = GameAbilities::Spectating(cur_map);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    #[should_panic(expected = "cannot mix playing and spectating state")]
    fn test_game_abilities_mixed_mode_panics() {
        let abilities = HashMap::new();
        let playing = GameAbilities::Playing(abilities);
        let spectating = GameAbilities::Spectating(HashMap::new());
        playing.diff(&spectating);
    }
}

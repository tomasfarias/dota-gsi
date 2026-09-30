use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{PlayerID, Team};

#[cfg(feature = "diff")]
use crate::diff::Diffable;
#[cfg(feature = "diff")]
use crate::event::{GameEvent, Hero as HeroEvent};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[cfg(feature = "diff")]
impl Diffable for Hero {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        /* IDs identify different heroes. Switching heroes (e.g. in demo or
        spectating) is not a level up. -1 means no hero is selected. */
        if self.id != new.id || self.id == -1 {
            return Vec::new();
        }

        match (self.level, new.level) {
            (Some(old), Some(level)) if level > old => {
                vec![GameEvent::HeroEvent(HeroEvent::LevelledUp {
                    id: new.id,
                    name: new.name.clone(),
                    level,
                })]
            }
            _ => Vec::new(),
        }
    }
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

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum GameHeroes {
    Spectating(HashMap<Team, HashMap<PlayerID, Hero>>),
    Playing(Hero),
}

#[cfg(feature = "diff")]
impl Diffable for GameHeroes {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        match (self, new) {
            (Self::Playing(old), Self::Playing(new)) => old.diff(new),
            (Self::Spectating(old), Self::Spectating(new)) => {
                let mut events = Vec::new();

                for (team, players) in old {
                    let Some(new_players) = new.get(team) else {
                        continue;
                    };

                    for (player_id, hero) in players {
                        if let Some(new_hero) = new_players.get(player_id) {
                            events.extend(hero.diff(new_hero));
                        }
                    }
                }

                events
            }
            _ => Vec::new(),
        }
    }
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

    #[cfg(feature = "diff")]
    mod diff_tests {
        use super::*;

        fn make_hero(id: i16, level: Option<u8>) -> Hero {
            serde_json::from_value(serde_json::json!({
                "id": id,
                "name": "npc_dota_hero_marci",
                "level": level,
            }))
            .expect("Failed to deserialize Hero")
        }

        #[test]
        fn test_hero_level_up() {
            let prev = make_hero(136, Some(1));

            for level in [2, 5] {
                let cur = make_hero(136, Some(level));
                assert_eq!(
                    prev.diff(&cur),
                    vec![GameEvent::HeroEvent(HeroEvent::LevelledUp {
                        id: 136,
                        name: Some("npc_dota_hero_marci".to_string()),
                        level,
                    })]
                );
            }
        }

        #[test]
        fn test_hero_without_level_increase_no_events() {
            for (old, new) in [
                (Some(2), Some(2)),
                (Some(2), Some(1)),
                (None, Some(2)),
                (Some(2), None),
                (None, None),
            ] {
                assert!(make_hero(136, old).diff(&make_hero(136, new)).is_empty());
            }
        }

        #[test]
        fn test_hero_changed_or_unselected_no_events() {
            for (old_id, new_id) in [(136, 2), (-1, 136), (136, -1), (-1, -1)] {
                let prev = make_hero(old_id, Some(1));
                let cur = make_hero(new_id, Some(2));
                assert!(prev.diff(&cur).is_empty());
            }
        }

        #[test]
        fn test_game_heroes_playing_level_up_without_name() {
            let mut hero = make_hero(136, Some(1));
            hero.name = None;
            let prev = GameHeroes::Playing(hero.clone());
            hero.level = Some(2);
            let cur = GameHeroes::Playing(hero);

            assert_eq!(
                prev.diff(&cur),
                vec![GameEvent::HeroEvent(HeroEvent::LevelledUp {
                    id: 136,
                    name: None,
                    level: 2,
                })]
            );
        }

        #[test]
        fn test_game_heroes_spectating_matches_players_and_skips_missing_entries() {
            let prev = GameHeroes::Spectating(HashMap::from([
                (
                    Team::Radiant,
                    HashMap::from([
                        (PlayerID(0), make_hero(136, Some(1))),
                        (PlayerID(1), make_hero(136, Some(8))),
                        (PlayerID(2), make_hero(136, Some(1))),
                    ]),
                ),
                (
                    Team::Dire,
                    HashMap::from([(PlayerID(5), make_hero(136, Some(1)))]),
                ),
            ]));
            let cur = GameHeroes::Spectating(HashMap::from([(
                Team::Radiant,
                HashMap::from([
                    (PlayerID(0), make_hero(136, Some(2))),
                    (PlayerID(1), make_hero(136, Some(8))),
                    (PlayerID(3), make_hero(136, Some(5))),
                ]),
            )]));

            assert_eq!(
                prev.diff(&cur),
                vec![GameEvent::HeroEvent(HeroEvent::LevelledUp {
                    id: 136,
                    name: Some("npc_dota_hero_marci".to_string()),
                    level: 2,
                })]
            );
        }

        #[test]
        fn test_game_heroes_mixed_mode_no_events() {
            let playing = GameHeroes::Playing(make_hero(136, Some(1)));
            let spectating = GameHeroes::Spectating(HashMap::new());

            assert!(playing.diff(&spectating).is_empty());
            assert!(spectating.diff(&playing).is_empty());
        }
    }
}

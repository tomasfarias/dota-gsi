use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize, de, de::Error, ser};
use thiserror;

use super::Team;

#[cfg(feature = "diff")]
use crate::diff::Diffable;
#[cfg(feature = "diff")]
use crate::event::{GameEvent, Player as PlayerEvent};

#[derive(thiserror::Error, Debug)]
pub enum PlayersError {
    #[error("failed to parse player ID number in `{0}`")]
    ParseIDError(String),
    #[error("attempted to parse an empty player")]
    EmptyPlayer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(from = "String")]
pub enum PlayerActivity {
    Menu,
    Playing,
    Undefined(String),
}

impl From<String> for PlayerActivity {
    fn from(s: String) -> Self {
        match s.as_str() {
            "menu" => PlayerActivity::Menu,
            "playing" => PlayerActivity::Playing,
            _ => PlayerActivity::Undefined(s),
        }
    }
}

impl fmt::Display for PlayerActivity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlayerActivity::Menu => write!(f, "In Menu"),
            PlayerActivity::Playing => write!(f, "Playing"),
            PlayerActivity::Undefined(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PlayerID(pub u8);

impl From<u8> for PlayerID {
    fn from(n: u8) -> Self {
        PlayerID(n)
    }
}

impl<'de> Deserialize<'de> for PlayerID {
    fn deserialize<D>(deserializer: D) -> Result<PlayerID, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut slot_split = s.split("player").map(|s| s.parse::<u8>());

        if let (_, Some(index_res)) = (slot_split.next(), slot_split.next()) {
            let index = index_res.map_err(D::Error::custom)?;
            return Ok(PlayerID(index));
        }

        Err(D::Error::custom(PlayersError::ParseIDError(s)))
    }
}

impl Serialize for PlayerID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&format!("player{}", self.0))
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Player {
    pub id: String,
    pub information: PlayerInformation,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PlayerInformation {
    pub steamid: String,
    pub name: String,
    pub activity: PlayerActivity,
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    pub last_hits: u16,
    pub denies: u16,
    pub kill_streak: u16,
    pub kill_list: HashMap<String, u32>,
    pub commands_issued: u32,
    pub team_name: Team,
    pub gold: u32,
    pub gold_reliable: u32,
    pub gold_unreliable: u32,
    pub gold_from_hero_kills: u32,
    pub gold_from_creep_kills: u32,
    pub gold_from_income: u32,
    pub gold_from_shared: u32,
    pub net_worth: Option<u32>,
    pub gpm: u32,
    pub xpm: u32,
}

#[cfg(feature = "diff")]
impl Diffable for PlayerInformation {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if self.kills < new.kills {
            events.push(GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                name: self.name.clone(),
                kills: new.kills,
                streak: new.kill_streak,
            }));
        }

        if self.deaths < new.deaths {
            events.push(GameEvent::PlayerEvent(PlayerEvent::Died {
                name: self.name.clone(),
                deaths: new.deaths,
            }));
        }

        if self.assists < new.assists {
            events.push(GameEvent::PlayerEvent(PlayerEvent::Assisted {
                name: self.name.clone(),
                assists: new.assists,
            }));
        }

        events
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.information.name)
    }
}

impl<'de> Deserialize<'de> for Player {
    fn deserialize<D>(deserializer: D) -> Result<Player, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let mut helper: HashMap<String, PlayerInformation> = HashMap::deserialize(deserializer)?;

        let (k, v) = helper
            .drain()
            .take(1)
            .next()
            .ok_or(PlayersError::EmptyPlayer)
            .map_err(D::Error::custom)?;

        Ok(Player {
            id: k,
            information: v,
        })
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum GamePlayers {
    Spectating(HashMap<Team, HashMap<PlayerID, PlayerInformation>>),
    Playing(PlayerInformation),
}

#[cfg(feature = "diff")]
impl Diffable for GamePlayers {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        match (self, new) {
            (GamePlayers::Spectating(current), GamePlayers::Spectating(new)) => {
                for (team, players) in current.iter() {
                    let Some(team_new) = new.get(team) else {
                        continue;
                    };

                    for (player_id, info) in players.iter() {
                        let Some(info_new) = team_new.get(player_id) else {
                            continue;
                        };

                        events.extend(info.diff(info_new));
                    }
                }
            }
            (GamePlayers::Playing(info), GamePlayers::Playing(info_new)) => {
                events.extend(info.diff(info_new));
            }
            _ => panic!(""),
        }

        events
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_players_deserialize() {
        let json_str = r#"{
    "team2": {
        "player0": {
            "activity": "playing",
            "assists": 5,
            "camps_stacked": 2,
            "commands_issued": 2138,
            "consumable_gold_spent": 1260,
            "deaths": 3,
            "denies": 3,
            "gold": 318,
            "gold_from_creep_kills": 288,
            "gold_from_hero_kills": 574,
            "gold_from_income": 1351,
            "gold_from_shared": 252,
            "gold_lost_to_death": 70,
            "gold_reliable": 102,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 216,
            "gpm": 202,
            "hero_damage": 2725,
            "item_gold_spent": 1650,
            "kill_list": {
              "victimid_5": 2
            },
            "kill_streak": 0,
            "kills": 2,
            "last_hits": 8,
            "name": "Nukkumatti",
            "net_worth": 2333,
            "runes_activated": 1,
            "steamid": "76561198069076692",
            "support_gold_spent": 250,
            "team_name": "radiant",
            "wards_destroyed": 1,
            "wards_placed": 3,
            "wards_purchased": 6,
            "xpm": 248
        },
            "player1": {
            "activity": "playing",
            "assists": 5,
            "camps_stacked": 0,
            "commands_issued": 4087,
            "consumable_gold_spent": 1205,
            "deaths": 4,
            "denies": 1,
            "gold": 219,
            "gold_from_creep_kills": 70,
            "gold_from_hero_kills": 167,
            "gold_from_income": 1351,
            "gold_from_shared": 167,
            "gold_lost_to_death": 48,
            "gold_reliable": 219,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 0,
            "gpm": 164,
            "hero_damage": 3750,
            "item_gold_spent": 1275,
            "kill_list": {},
            "kill_streak": 0,
            "kills": 0,
            "last_hits": 4,
            "name": "Keral",
            "net_worth": 1999,
            "runes_activated": 1,
            "steamid": "76561198122362484",
            "support_gold_spent": 425,
            "team_name": "radiant",
            "wards_destroyed": 1,
            "wards_placed": 6,
            "wards_purchased": 18,
            "xpm": 196
        },
        "player2": {
            "activity": "playing",
            "assists": 5,
            "camps_stacked": 1,
            "commands_issued": 3910,
            "consumable_gold_spent": 390,
            "deaths": 1,
            "denies": 10,
            "gold": 744,
            "gold_from_creep_kills": 2552,
            "gold_from_hero_kills": 215,
            "gold_from_income": 1351,
            "gold_from_shared": 215,
            "gold_lost_to_death": 26,
            "gold_reliable": 374,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 370,
            "gpm": 371,
            "hero_damage": 1965,
            "item_gold_spent": 4275,
            "kill_list": {},
            "kill_streak": 0,
            "kills": 0,
            "last_hits": 70,
            "name": "day",
            "net_worth": 5219,
            "runes_activated": 2,
            "steamid": "76561198259369550",
            "support_gold_spent": 0,
            "team_name": "radiant",
            "wards_destroyed": 0,
            "wards_placed": 0,
            "wards_purchased": 0,
            "xpm": 365
        },
        "player3": {
            "activity": "playing",
            "assists": 1,
            "camps_stacked": 1,
            "commands_issued": 4597,
            "consumable_gold_spent": 460,
            "deaths": 2,
            "denies": 5,
            "gold": 630,
            "gold_from_creep_kills": 2197,
            "gold_from_hero_kills": 1929,
            "gold_from_income": 1351,
            "gold_from_shared": 593,
            "gold_lost_to_death": 246,
            "gold_reliable": 317,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 313,
            "gpm": 469,
            "hero_damage": 10743,
            "item_gold_spent": 5375,
            "kill_list": {
              "victimid_5": 3,
              "victimid_6": 2,
              "victimid_7": 1,
              "victimid_9": 2
            },
            "kill_streak": 0,
            "kills": 8,
            "last_hits": 66,
            "name": "MUTE 48(46) HOURS",
            "net_worth": 6205,
            "runes_activated": 5,
            "steamid": "76561198312019784",
            "support_gold_spent": 0,
            "team_name": "radiant",
            "wards_destroyed": 0,
            "wards_placed": 2,
            "wards_purchased": 2,
            "xpm": 509
        },
        "player4": {
            "activity": "playing",
            "assists": 0,
            "camps_stacked": 0,
            "commands_issued": 4157,
            "consumable_gold_spent": 190,
            "deaths": 1,
            "denies": 11,
            "gold": 425,
            "gold_from_creep_kills": 3006,
            "gold_from_hero_kills": 197,
            "gold_from_income": 1351,
            "gold_from_shared": 56,
            "gold_lost_to_death": 81,
            "gold_reliable": 120,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 305,
            "gpm": 421,
            "hero_damage": 3511,
            "item_gold_spent": 5400,
            "kill_list": {
              "victimid_9": 1
            },
            "kill_streak": 0,
            "kills": 1,
            "last_hits": 95,
            "name": "d?e",
            "net_worth": 6025,
            "runes_activated": 1,
            "steamid": "76561198313867774",
            "support_gold_spent": 0,
            "team_name": "radiant",
            "wards_destroyed": 0,
            "wards_placed": 0,
            "wards_purchased": 0,
            "xpm": 443
        }
    },
    "team3": {
        "player5": {
            "activity": "playing",
            "assists": 5,
            "camps_stacked": 1,
            "commands_issued": 3107,
            "consumable_gold_spent": 1660,
            "deaths": 6,
            "denies": 0,
            "gold": 99,
            "gold_from_creep_kills": 24,
            "gold_from_hero_kills": 1009,
            "gold_from_income": 1351,
            "gold_from_shared": 343,
            "gold_lost_to_death": 99,
            "gold_reliable": 99,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 0,
            "gpm": 248,
            "hero_damage": 6394,
            "item_gold_spent": 2330,
            "kill_list": {
              "victimid_0": 1,
              "victimid_2": 1,
              "victimid_3": 1
            },
            "kill_streak": 0,
            "kills": 3,
            "last_hits": 11,
            "name": "><><",
            "net_worth": 2504,
            "runes_activated": 0,
            "steamid": "76561198300389107",
            "support_gold_spent": 500,
            "team_name": "dire",
            "wards_destroyed": 3,
            "wards_placed": 8,
            "wards_purchased": 19,
            "xpm": 238
        },
        "player6": {
            "activity": "playing",
            "assists": 2,
            "camps_stacked": 0,
            "commands_issued": 4546,
            "consumable_gold_spent": 680,
            "deaths": 2,
            "denies": 2,
            "gold": 379,
            "gold_from_creep_kills": 2701,
            "gold_from_hero_kills": 735,
            "gold_from_income": 1351,
            "gold_from_shared": 248,
            "gold_lost_to_death": 0,
            "gold_reliable": 107,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 272,
            "gpm": 438,
            "hero_damage": 6775,
            "item_gold_spent": 6000,
            "kill_list": {
              "victimid_0": 1,
              "victimid_1": 1,
              "victimid_4": 1
            },
            "kill_streak": 0,
            "kills": 3,
            "last_hits": 84,
            "name": "SabeRLighT-",
            "net_worth": 5704,
            "runes_activated": 1,
            "steamid": "76561198086478594",
            "support_gold_spent": 0,
            "team_name": "dire",
            "wards_destroyed": 0,
            "wards_placed": 0,
            "wards_purchased": 0,
            "xpm": 490
        },
        "player7": {
            "activity": "playing",
            "assists": 3,
            "camps_stacked": 2,
            "commands_issued": 4609,
            "consumable_gold_spent": 290,
            "deaths": 1,
            "denies": 11,
            "gold": 342,
            "gold_from_creep_kills": 1436,
            "gold_from_hero_kills": 735,
            "gold_from_income": 1351,
            "gold_from_shared": 248,
            "gold_lost_to_death": 52,
            "gold_reliable": 185,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 157,
            "gpm": 428,
            "hero_damage": 5843,
            "item_gold_spent": 5530,
            "kill_list": {
              "victimid_0": 1,
              "victimid_1": 2
            },
            "kill_streak": 3,
            "kills": 3,
            "last_hits": 94,
            "name": "Stfu all",
            "net_worth": 6022,
            "runes_activated": 2,
            "steamid": "76561198990897157",
            "support_gold_spent": 0,
            "team_name": "dire",
            "wards_destroyed": 1,
            "wards_placed": 0,
            "wards_purchased": 0,
            "xpm": 473
        },
        "player8": {
            "activity": "playing",
            "assists": 3,
            "camps_stacked": 0,
            "commands_issued": 3129,
            "consumable_gold_spent": 310,
            "deaths": 0,
            "denies": 7,
            "gold": 15,
            "gold_from_creep_kills": 2658,
            "gold_from_hero_kills": 153,
            "gold_from_income": 1351,
            "gold_from_shared": 153,
            "gold_lost_to_death": 0,
            "gold_reliable": 15,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 0,
            "gpm": 364,
            "hero_damage": 4261,
            "item_gold_spent": 5025,
            "kill_list": {},
            "kill_streak": 0,
            "kills": 0,
            "last_hits": 72,
            "name": "Hook",
            "net_worth": 5290,
            "runes_activated": 5,
            "steamid": "76561198397077737",
            "support_gold_spent": 0,
            "team_name": "dire",
            "wards_destroyed": 0,
            "wards_placed": 2,
            "wards_purchased": 3,
            "xpm": 456
        },
        "player9": {
            "activity": "playing",
            "assists": 2,
            "camps_stacked": 1,
            "commands_issued": 3728,
            "consumable_gold_spent": 365,
            "deaths": 3,
            "denies": 0,
            "gold": 621,
            "gold_from_creep_kills": 626,
            "gold_from_hero_kills": 686,
            "gold_from_income": 1351,
            "gold_from_shared": 166,
            "gold_lost_to_death": 147,
            "gold_reliable": 407,
            "gold_spent_on_buybacks": 0,
            "gold_unreliable": 214,
            "gpm": 282,
            "hero_damage": 3876,
            "item_gold_spent": 3325,
            "kill_list": {
              "victimid_1": 1,
              "victimid_3": 1
            },
            "kill_streak": 1,
            "kills": 2,
            "last_hits": 34,
            "name": "Kaito",
            "net_worth": 4021,
            "runes_activated": 2,
            "steamid": "76561198010162548",
            "support_gold_spent": 125,
            "team_name": "dire",
            "wards_destroyed": 0,
            "wards_placed": 1,
            "wards_purchased": 4,
            "xpm": 322
        }
    }
}"#;

        let players: GamePlayers =
            serde_json::from_str(json_str).expect("Failed to deserialize Players");

        assert!(matches!(players, GamePlayers::Spectating(_)));
    }

    #[test]
    fn test_player_activity_from_str() {
        assert!(matches!(
            PlayerActivity::from("menu".to_string()),
            PlayerActivity::Menu
        ));
        assert!(matches!(
            PlayerActivity::from("playing".to_string()),
            PlayerActivity::Playing
        ));
    }

    pub(crate) fn make_player_info(
        kills: u16,
        deaths: u16,
        assists: u16,
        streak: u16,
    ) -> PlayerInformation {
        PlayerInformation {
            steamid: "76561198000000000".to_string(),
            name: "TestPlayer".to_string(),
            activity: PlayerActivity::Playing,
            kills,
            deaths,
            assists,
            last_hits: 0,
            denies: 0,
            kill_streak: streak,
            kill_list: HashMap::new(),
            commands_issued: 0,
            team_name: Team::Radiant,
            gold: 600,
            gold_reliable: 0,
            gold_unreliable: 600,
            gold_from_hero_kills: 0,
            gold_from_creep_kills: 0,
            gold_from_income: 0,
            gold_from_shared: 0,
            net_worth: None,
            gpm: 0,
            xpm: 0,
        }
    }

    #[test]
    fn test_player_info_no_change() {
        let info = make_player_info(0, 0, 0, 0);
        let events = info.diff(&info.clone());
        assert!(events.is_empty());
    }

    #[test]
    fn test_player_info_secured_kill() {
        let prev = make_player_info(2, 0, 0, 2);
        let cur = make_player_info(3, 0, 0, 3);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                name: "TestPlayer".to_owned(),
                kills: 3,
                streak: 3
            })]
        );
    }

    #[test]
    fn test_player_info_died() {
        let prev = make_player_info(0, 1, 0, 0);
        let cur = make_player_info(0, 2, 0, 0);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::PlayerEvent(PlayerEvent::Died {
                name: "TestPlayer".to_owned(),
                deaths: 2
            })]
        );
    }

    #[test]
    fn test_player_info_assisted() {
        let prev = make_player_info(0, 0, 0, 0);
        let cur = make_player_info(0, 0, 1, 0);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::PlayerEvent(PlayerEvent::Assisted {
                name: "TestPlayer".to_owned(),
                assists: 1
            })]
        );
    }

    #[test]
    fn test_player_info_kill_and_death_same_tick() {
        let prev = make_player_info(2, 1, 0, 2);
        let cur = make_player_info(3, 2, 0, 1);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![
                GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                    name: "TestPlayer".to_owned(),
                    kills: 3,
                    streak: 1
                }),
                GameEvent::PlayerEvent(PlayerEvent::Died {
                    name: "TestPlayer".to_owned(),
                    deaths: 2
                }),
            ]
        );
    }

    #[test]
    fn test_player_info_multi_kill_single_tick() {
        let prev = make_player_info(0, 0, 0, 0);
        let cur = make_player_info(3, 0, 0, 3);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                name: "TestPlayer".to_owned(),
                kills: 3,
                streak: 3
            })]
        );
    }
    #[test]
    fn test_game_players_playing_kill() {
        let prev = GamePlayers::Playing(make_player_info(0, 0, 0, 0));
        let cur = GamePlayers::Playing(make_player_info(1, 0, 0, 1));
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                name: "TestPlayer".to_owned(),
                kills: 1,
                streak: 1
            })]
        );
    }

    #[test]
    fn test_game_players_playing_no_change() {
        let info = make_player_info(5, 2, 3, 1);
        let prev = GamePlayers::Playing(info.clone());
        let cur = GamePlayers::Playing(info);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_game_players_spectating_multiple_deaths() {
        let mut radiant_prev = HashMap::new();
        radiant_prev.insert(PlayerID(0), make_player_info(0, 0, 0, 0));
        radiant_prev.insert(PlayerID(1), make_player_info(0, 0, 0, 0));

        let mut radiant_cur = HashMap::new();
        radiant_cur.insert(PlayerID(0), make_player_info(0, 1, 0, 0));
        radiant_cur.insert(PlayerID(1), make_player_info(0, 1, 0, 0));

        let mut prev_map = HashMap::new();
        prev_map.insert(Team::Radiant, radiant_prev);
        let mut cur_map = HashMap::new();
        cur_map.insert(Team::Radiant, radiant_cur);

        let prev = GamePlayers::Spectating(prev_map);
        let cur = GamePlayers::Spectating(cur_map);
        let events = prev.diff(&cur);

        // Both players died — should have 2 death events
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| matches!(
            e,
            GameEvent::PlayerEvent(PlayerEvent::Died { name: _, deaths: 1 })
        )));
    }

    #[test]
    fn test_game_players_spectating_missing_player_no_panic() {
        let mut radiant_prev = HashMap::new();
        radiant_prev.insert(PlayerID(0), make_player_info(0, 0, 0, 0));
        radiant_prev.insert(PlayerID(1), make_player_info(0, 0, 0, 0));

        // Player 1 gone in new state
        let mut radiant_cur = HashMap::new();
        radiant_cur.insert(PlayerID(0), make_player_info(0, 0, 0, 0));

        let mut prev_map = HashMap::new();
        prev_map.insert(Team::Radiant, radiant_prev);
        let mut cur_map = HashMap::new();
        cur_map.insert(Team::Radiant, radiant_cur);

        let prev = GamePlayers::Spectating(prev_map);
        let cur = GamePlayers::Spectating(cur_map);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    #[should_panic]
    fn test_game_players_mixed_mode_panics() {
        let playing = GamePlayers::Playing(make_player_info(0, 0, 0, 0));
        let spectating = GamePlayers::Spectating(HashMap::new());
        playing.diff(&spectating);
    }
}

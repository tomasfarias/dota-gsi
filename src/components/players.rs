use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize, de, de::Error, ser};
use thiserror;

use super::Team;

#[derive(thiserror::Error, Debug)]
pub enum PlayersError {
    #[error("failed to parse player ID number in `{0}`")]
    ParseIDError(String),
    #[error("attempted to parse an empty player")]
    EmptyPlayer,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct PlayerID(u8);

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

#[derive(Serialize, Debug)]
pub struct Player {
    pub id: String,
    pub information: PlayerInformation,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum GamePlayers {
    Spectating(HashMap<Team, HashMap<PlayerID, PlayerInformation>>),
    Playing(PlayerInformation),
}

#[cfg(test)]
mod tests {
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
}

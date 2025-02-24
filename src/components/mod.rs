use std::collections::HashMap;
use std::fmt;

use serde::{de, de::Error, Deserialize, Serialize};
use serde_json::{map, Value};

pub mod abilities;
pub mod buildings;
pub mod heroes;
pub mod items;
pub mod players;
pub mod team;
pub mod wearables;

use abilities::GameAbilities;
use buildings::Buildings;
use heroes::{GameHeroes, Hero};
use items::{GameItems, Items};
use players::{GamePlayers, PlayerID};
use team::Team;
use wearables::GameWearables;

/// Represents Game State Integration authentication via an optional token
#[derive(Serialize, Deserialize, Debug)]
pub struct Auth {
    token: Option<String>,
}

/// An enum of all possible GAMERULES states
#[derive(Serialize, Deserialize, Debug)]
#[serde(from = "String")]
pub enum DotaGameRulesState {
    Disconnected,
    InProgress,
    HeroSelection,
    Starting,
    Ending,
    PostGame,
    PreGame,
    StrategyTime,
    WaitingForMap,
    WaitingForPlayers,
    CustomGameSetup,
    Undefined(String),
}

impl From<String> for DotaGameRulesState {
    fn from(s: String) -> Self {
        match s.as_str() {
            "DOTA_GAMERULES_STATE_DISCONNECT" => DotaGameRulesState::Disconnected,
            "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS" => DotaGameRulesState::InProgress,
            "DOTA_GAMERULES_STATE_HERO_SELECTION" => DotaGameRulesState::HeroSelection,
            "DOTA_GAMERULES_STATE_INIT" => DotaGameRulesState::Starting,
            "DOTA_GAMERULES_STATE_LAST" => DotaGameRulesState::Ending,
            "DOTA_GAMERULES_STATE_POST_GAME" => DotaGameRulesState::PostGame,
            "DOTA_GAMERULES_STATE_PRE_GAME" => DotaGameRulesState::PreGame,
            "DOTA_GAMERULES_STATE_STRATEGY_TIME" => DotaGameRulesState::StrategyTime,
            "DOTA_GAMERULES_STATE_WAIT_FOR_MAP_TO_LOAD" => DotaGameRulesState::WaitingForMap,
            "DOTA_GAMERULES_STATE_WAIT_FOR_PLAYERS_TO_LOAD" => {
                DotaGameRulesState::WaitingForPlayers
            }
            "DOTA_GAMERULES_STATE_CUSTOM_GAME_SETUP" => DotaGameRulesState::CustomGameSetup,
            _ => DotaGameRulesState::Undefined(s),
        }
    }
}

impl fmt::Display for DotaGameRulesState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DotaGameRulesState::Disconnected => write!(f, "Disconnected"),
            DotaGameRulesState::InProgress => write!(f, "In Progress"),
            DotaGameRulesState::HeroSelection => write!(f, "Hero Selection"),
            DotaGameRulesState::Starting => write!(f, "Starting"),
            DotaGameRulesState::Ending => write!(f, "Ending"),
            DotaGameRulesState::PostGame => write!(f, "Post Game"),
            DotaGameRulesState::PreGame => write!(f, "Pre Game"),
            DotaGameRulesState::StrategyTime => write!(f, "Strategy Time"),
            DotaGameRulesState::WaitingForMap => write!(f, "Waiting For Map"),
            DotaGameRulesState::WaitingForPlayers => write!(f, "Waiting For Players"),
            DotaGameRulesState::CustomGameSetup => write!(f, "Custom Game Setup"),
            DotaGameRulesState::Undefined(s) => write!(f, "Undefined: {}", s),
        }
    }
}

/// The Game State Integration provider, will be Dota
#[derive(Serialize, Deserialize, Debug)]
pub struct Provider {
    name: String,
    #[serde(alias = "appid")]
    app_id: u32,
    version: u32,
    timestamp: u32,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version)
    }
}

/// Represents a Dota Game State Integration map
#[derive(Serialize, Deserialize, Debug)]
pub struct Map {
    name: String,
    #[serde(alias = "matchid")]
    match_id: String,
    game_time: u32,
    clock_time: i32,
    daytime: bool,
    nightstalker_night: bool,
    game_state: DotaGameRulesState,
    paused: bool,
    win_team: Team,
    customgamename: String,
    ward_purchase_cooldown: Option<u16>,
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Match ID: {}\nState: {}\nMap: {}\nTime: {}\n",
            self.match_id, self.game_state, self.name, self.game_time
        )
    }
}

fn empty_map_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: de::Deserializer<'de>,
    T: de::DeserializeOwned + std::fmt::Debug,
{
    let opt = Option::<map::Map<String, Value>>::deserialize(de)?;

    match opt {
        None => Ok(None),
        Some(m) => {
            if m.is_empty() {
                Ok(None)
            } else {
                let res: T = serde_json::from_value(Value::Object(m)).map_err(D::Error::custom)?;
                Ok(Some(res))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameState {
    provider: Provider,
    #[serde(default, deserialize_with = "empty_map_as_none")]
    buildings: Option<HashMap<Team, Buildings>>,
    map: Option<Map>,
    #[serde(alias = "player", default, deserialize_with = "empty_map_as_none")]
    players: Option<GamePlayers>,
    #[serde(alias = "hero", default, deserialize_with = "empty_map_as_none")]
    heroes: Option<GameHeroes>,
    #[serde(default, deserialize_with = "empty_map_as_none")]
    abilities: Option<GameAbilities>,
    #[serde(default, deserialize_with = "empty_map_as_none")]
    items: Option<GameItems>,
    draft: Option<HashMap<Team, HashMap<PlayerID, Value>>>,
    #[serde(default, deserialize_with = "empty_map_as_none")]
    wearables: Option<GameWearables>,
}

impl GameState {
    pub fn get_items(&self) -> Option<&Items> {
        if let Some(items) = &self.items {
            match items {
                GameItems::Playing(i) => Some(i),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn get_heroes(&self) -> Option<&GameHeroes> {
        self.heroes.as_ref()
    }

    pub fn get_players(&self) -> Option<&GameHeroes> {
        self.heroes.as_ref()
    }

    pub fn get_hero(&self) -> Option<&Hero> {
        if let Some(heroes) = &self.heroes {
            match heroes {
                GameHeroes::Playing(h) => Some(h),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn get_team_player_items(&self, team: &Team, id: &PlayerID) -> Option<&Items> {
        if let Some(items) = &self.items {
            match items {
                GameItems::Spectating(m) => match m.get(team) {
                    Some(t) => t.get(id),
                    None => None,
                },
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn get_team_player_hero(&self, team: &Team, id: &PlayerID) -> Option<&Hero> {
        if let Some(heroes) = &self.heroes {
            match heroes {
                GameHeroes::Spectating(m) => match m.get(team) {
                    Some(t) => t.get(id),
                    None => None,
                },
                _ => None,
            }
        } else {
            None
        }
    }
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.provider)?;

        if let Some(map) = &self.map {
            writeln!(f, "{}", map)?;
        }

        if let Some(players) = &self.players {
            match players {
                GamePlayers::Playing(p) => {
                    writeln!(f, "{}\n{}", p.team_name, p.name)?;

                    if let Some(hero) = self.get_hero() {
                        writeln!(f, "{}", hero)?;
                    }

                    if let Some(items) = self.get_items() {
                        writeln!(f, "{}", items)?;
                    }
                }
                GamePlayers::Spectating(i) => {
                    for (team, players) in i.iter() {
                        writeln!(f, "{}", team)?;
                        for (id, player) in players.iter() {
                            writeln!(f, "{}", player.name)?;

                            if let Some(hero) = self.get_team_player_hero(team, id) {
                                writeln!(f, "{}", hero)?;
                            }

                            if let Some(items) = self.get_team_player_items(team, id) {
                                writeln!(f, "{}", items)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_game_state_deserialize() {
        let json_str = r#"{
            "provider": {
                "name": "Dota 2",
                "appid": 570,
                "version": 47,
                "timestamp": 1658690112
            },
            "player": {},
            "draft": {},
            "auth": {
                "token": "1234"
            }
        }"#;
        let gs: GameState =
            serde_json::from_str(json_str).expect("Failed to deserialize GameState");

        assert!(gs.players.is_none());
        assert!(gs.map.is_none());
        assert!(gs.heroes.is_none());
        assert_eq!(gs.provider.name, "Dota 2".to_owned());
    }

    #[test]
    fn test_inititalizing_game_state_deserialize() {
        let json_str = r#"{
    "buildings": {
        "radiant": {
            "dota_goodguys_tower1_mid": {
                "health": 1800,
                "max_health": 1800
            }
        },
        "dire": {
            "dota_badguys_tower1_mid": {
                "health": 1800,
                "max_health": 1800
            }
        }
    },
    "provider": {
        "name": "Dota 2",
        "appid": 570,
        "version": 47,
        "timestamp": 1659017150
    },
    "map": {
        "name": "hero_demo_main",
        "matchid": "0",
        "game_time": 1,
        "clock_time": 1,
        "daytime": true,
        "nightstalker_night": false,
        "game_state": "DOTA_GAMERULES_STATE_INIT",
        "paused": false,
        "win_team": "none",
        "customgamename": "/.local/share/Steam/steamapps/common/dota 2 beta/game/dota_addons/hero_demo"
    },
    "player": {},
    "hero": {},
    "abilities": {},
    "items": {},
    "draft": {},
    "wearables": {},
    "auth": {
        "token": "hello1234"
    }
}"#;
        let gs: GameState =
            serde_json::from_str(json_str).expect("Failed to deserialize GameState starting");
        let buildings = gs.buildings.unwrap();

        assert!(matches!(
            gs.map.unwrap().game_state,
            DotaGameRulesState::Starting
        ));
        assert_eq!(buildings.is_empty(), false);
        assert_eq!(buildings.len(), 2);
    }

    #[test]
    fn test_strategy_time_game_state_deserialize() {
        let json_str = r#"{
    "buildings": {
        "radiant": {
            "dota_goodguys_tower1_mid": {
                "health": 1800,
                "max_health": 1800
            }
        }
    },
    "provider": {
        "name": "Dota 2",
        "appid": 570,
        "version": 47,
        "timestamp": 1659033793
    },
    "map": {
        "name": "hero_demo_main",
        "matchid": "0",
        "game_time": 1,
        "clock_time": 0,
        "daytime": true,
        "nightstalker_night": false,
        "game_state": "DOTA_GAMERULES_STATE_STRATEGY_TIME",
        "paused": false,
        "win_team": "none",
        "customgamename": "/home/tomasfarias/.local/share/Steam/steamapps/common/dota 2 beta/game/dota_addons/hero_demo",
        "ward_purchase_cooldown": 0
    },
    "player": {
        "steamid": "76561197996881999",
        "name": "farxc3xadas",
        "activity": "playing",
        "kills": 0,
        "deaths": 0,
        "assists": 0,
        "last_hits": 0,
        "denies": 0,
        "kill_streak": 0,
        "commands_issued": 0,
        "kill_list": {},
        "team_name": "radiant",
        "gold": 600,
        "gold_reliable": 0,
        "gold_unreliable": 600,
        "gold_from_hero_kills": 0,
        "gold_from_creep_kills": 0,
        "gold_from_income": 0,
        "gold_from_shared": 0,
        "gpm": 0,
        "xpm": 0
    },
    "hero": {
        "id": 90,
        "name": "npc_dota_hero_keeper_of_the_light"
    },
    "abilities": {},
    "items": {},
    "draft": {},
    "wearables": {
        "wearable0": 13773,
        "wearable1": 14451,
        "wearable2": 14452,
        "wearable3": 14450,
        "wearable4": 12433,
        "wearable5": 528
    },
    "auth": {"token": "hello1234"}
}"#;
        let gs: GameState =
            serde_json::from_str(json_str).expect("Failed to deserialize GameState Strategy Time");

        assert!(matches!(
            gs.map.unwrap().game_state,
            DotaGameRulesState::StrategyTime
        ));
    }

    #[test]
    fn test_in_progress_game_state_deserialize() {
        let json_str = r#"{
  "buildings": {
    "radiant": {
      "dota_goodguys_tower1_mid": {
        "health": 1800,
        "max_health": 1800
      }
    }
  },
  "provider": {
    "name": "Dota 2",
    "appid": 570,
    "version": 47,
    "timestamp": 1659035016
  },
  "map": {
    "name": "hero_demo_main",
    "matchid": "0",
    "game_time": 1,
    "clock_time": 0,
    "daytime": true,
    "nightstalker_night": false,
    "game_state": "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS",
    "paused": false,
    "win_team": "none",
    "customgamename": "/home/tomasfarias/.local/share/Steam/steamapps/common/dota 2 beta/game/dota_addons/hero_demo",
    "ward_purchase_cooldown": 0
  },
  "player": {
    "steamid": "76561197996881999",
    "name": "farxc3xadas",
    "activity": "playing",
    "kills": 0,
    "deaths": 0,
    "assists": 0,
    "last_hits": 0,
    "denies": 0,
    "kill_streak": 0,
    "commands_issued": 0,
    "kill_list": {},
    "team_name": "radiant",
    "gold": 600,
    "gold_reliable": 0,
    "gold_unreliable": 600,
    "gold_from_hero_kills": 0,
    "gold_from_creep_kills": 0,
    "gold_from_income": 0,
    "gold_from_shared": 0,
    "gpm": 0,
    "xpm": 0
  },
  "hero": {
    "xpos": -1664,
    "ypos": -1216,
    "id": 42,
    "name": "npc_dota_hero_skeleton_king",
    "level": 0,
    "xp": 0,
    "alive": false,
    "respawn_seconds": 0,
    "buyback_cost": 200,
    "buyback_cooldown": 0,
    "health": 640,
    "max_health": 640,
    "health_percent": 100,
    "mana": 291,
    "max_mana": 291,
    "mana_percent": 100,
    "silenced": false,
    "stunned": false,
    "disarmed": false,
    "magicimmune": false,
    "hexed": false,
    "muted": false,
    "break": false,
    "aghanims_scepter": false,
    "aghanims_shard": false,
    "smoked": false,
    "has_debuff": false,
    "talent_1": false,
    "talent_2": false,
    "talent_3": false,
    "talent_4": false,
    "talent_5": false,
    "talent_6": false,
    "talent_7": false,
    "talent_8": false
  },
  "abilities": {
    "ability0": {
      "name": "skeleton_king_hellfire_blast",
      "level": 0,
      "can_cast": false,
      "passive": false,
      "ability_active": true,
      "cooldown": 0,
      "ultimate": false
    },
    "ability1": {
      "name": "skeleton_king_vampiric_aura",
      "level": 0,
      "can_cast": false,
      "passive": false,
      "ability_active": true,
      "cooldown": 0,
      "ultimate": false
    },
    "ability2": {
      "name": "skeleton_king_mortal_strike",
      "level": 0,
      "can_cast": false,
      "passive": true,
      "ability_active": true,
      "cooldown": 0,
      "ultimate": false
    },
    "ability3": {
      "name": "skeleton_king_reincarnation",
      "level": 0,
      "can_cast": false,
      "passive": true,
      "ability_active": true,
      "cooldown": 0,
      "ultimate": true
    },
    "ability4": {
      "name": "plus_high_five",
      "level": 1,
      "can_cast": true,
      "passive": false,
      "ability_active": true,
      "cooldown": 0,
      "ultimate": false
    },
    "ability5": {
      "name": "plus_guild_banner",
      "level": 1,
      "can_cast": true,
      "passive": false,
      "ability_active": true,
      "cooldown": 0,
      "ultimate": false
    }
  },
  "items": {
    "slot0": {
      "name": "empty"
    },
    "slot1": {
        "name": "item_manta",
        "purchaser": 0,
        "can_cast": true,
        "cooldown": 0,
        "passive": false
    },
    "slot2": {
      "name": "item_ultimate_orb",
      "purchaser": 0,
      "passive": true
    },
    "slot3": {
      "name": "empty"
    },
    "slot4": {
      "name": "empty"
    },
    "slot5": {
      "name": "empty"
    },
    "slot6": {
      "name": "empty"
    },
    "slot7": {
      "name": "empty"
    },
    "slot8": {
      "name": "empty"
    },
    "stash0": {
      "name": "empty"
    },
    "stash1": {
      "name": "empty"
    },
    "stash2": {
      "name": "empty"
    },
    "stash3": {
      "name": "empty"
    },
    "stash4": {
      "name": "empty"
    },
    "stash5": {
      "name": "empty"
    },
    "teleport0": {
      "name": "item_tpscroll",
      "purchaser": 0,
      "can_cast": false,
      "cooldown": 100,
      "passive": false,
      "charges": 1
    },
    "neutral0": {
      "name": "empty"
    }
  },
  "draft": {},
  "wearables": {
    "wearable0": 9747,
    "wearable1": 8780,
    "wearable2": 8623,
    "wearable3": 8622,
    "wearable4": 8624,
    "wearable5": 14942,
    "wearable6": 483,
    "wearable7": 8621,
    "wearable8": 790,
    "wearable9": 792,
    "wearable10": 791,
    "wearable11": 14912
  },
  "auth": {
    "token": "hello1234"
  }
}"#;

        let gs: GameState =
            serde_json::from_str(json_str).expect("Failed to deserialize GameState In Progress");
        let heroes = gs.heroes.as_ref().unwrap();
        let wearables = gs.wearables.as_ref().unwrap();
        let players = gs.players.as_ref().unwrap();

        assert!(matches!(
            gs.map.as_ref().unwrap().game_state,
            DotaGameRulesState::InProgress,
        ));

        assert!(matches!(heroes, GameHeroes::Playing(_)));
        if let GameHeroes::Playing(hero) = heroes {
            assert_eq!(hero.id, 42);
        } else {
            panic!("Failed to deserialize single hero");
        }

        assert!(matches!(wearables, GameWearables::Playing(_)));
        if let GameWearables::Playing(wearables_map) = wearables {
            assert_eq!(wearables_map.len(), 12);
        } else {
            panic!("Failed to deserialize wearables");
        }

        assert!(matches!(players, GamePlayers::Playing(_)));
        assert!(gs.get_items().is_some());
    }

    #[test]
    fn test_map_deserialize() {
        let json_str = r#"{
            "name": "hero_demo_main",
            "matchid": "0",
            "game_time": 5,
            "clock_time": 4,
            "daytime": true,
            "nightstalker_night": false,
            "game_state": "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS",
            "paused": false,
            "win_team": "none",
            "customgamename": "common/dota 2 beta/game/dota_addons/hero_demo",
            "ward_purchase_cooldown": 0
        }"#;

        let map: Map = serde_json::from_str(json_str).expect("Failed to deserialize Map");

        assert_eq!(map.name, "hero_demo_main");
        assert_eq!(map.match_id, "0");
        assert_eq!(map.game_time, 5);
        assert_eq!(map.clock_time, 4);
        assert_eq!(map.daytime, true);
        assert_eq!(map.nightstalker_night, false);
        assert!(matches!(map.game_state, DotaGameRulesState::InProgress));
        assert_eq!(map.paused, false);
    }
}

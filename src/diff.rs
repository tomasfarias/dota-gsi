/// Compare two Dota 2 game states to produce game events
///
/// This provides a [`Diffable`] trait implemented for game [`crate::components`].
use crate::components::{self, GameState, abilities, players};
use crate::event::{Ability, GameEvent, Map, Player};

/// Diffable trait to compare a game component with a newer one
pub trait Diffable {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent>;
}

impl Diffable for abilities::Ability {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if self.level < new.level {
            events.push(GameEvent::AbilityEvent(Ability::LevelledUp(new.level)));
        }

        match (self.can_cast, new.can_cast) {
            (true, false) => events.push(GameEvent::AbilityEvent(Ability::WentOnCooldown(
                new.cooldown,
            ))),
            (false, true) => events.push(GameEvent::AbilityEvent(Ability::WentOffCooldown)),
            _ => {}
        }

        match (self.ability_active, new.ability_active) {
            (true, false) => events.push(GameEvent::AbilityEvent(Ability::Deactivated)),
            (false, true) => events.push(GameEvent::AbilityEvent(Ability::Activated)),
            _ => {}
        }

        events
    }
}

impl Diffable for abilities::GameAbilities {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        match (self, new) {
            (
                abilities::GameAbilities::Spectating(current),
                abilities::GameAbilities::Spectating(new),
            ) => {
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
            (
                abilities::GameAbilities::Playing(abilities),
                abilities::GameAbilities::Playing(abilities_new),
            ) => {
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

impl Diffable for players::PlayerInformation {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if self.kills < new.kills {
            events.push(GameEvent::PlayerEvent(Player::SecuredKill {
                kills: new.kills,
                streak: new.kill_streak,
            }));
        }

        if self.deaths < new.deaths {
            events.push(GameEvent::PlayerEvent(Player::Died(new.deaths)));
        }

        if self.assists < new.assists {
            events.push(GameEvent::PlayerEvent(Player::Assisted(new.assists)));
        }

        events
    }
}

impl Diffable for players::GamePlayers {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        match (self, new) {
            (players::GamePlayers::Spectating(current), players::GamePlayers::Spectating(new)) => {
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
            (players::GamePlayers::Playing(info), players::GamePlayers::Playing(info_new)) => {
                events.extend(info.diff(info_new));
            }
            _ => panic!(""),
        }

        events
    }
}

impl Diffable for components::Map {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        match (self.daytime, new.daytime) {
            (true, false) => events.push(GameEvent::MapEvent(Map::StartedNight {
                nightstalker: new.nightstalker_night,
            })),
            (false, true) => events.push(GameEvent::MapEvent(Map::StartedDay)),
            _ => {}
        }

        events
    }
}

impl Diffable for GameState {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if let (Some(dota_map), Some(dota_map_new)) = (self.map.as_ref(), new.map.as_ref()) {
            events.extend(dota_map.diff(dota_map_new));
        }

        if let (Some(abilities), Some(abilities_new)) =
            (self.abilities.as_ref(), new.abilities.as_ref())
        {
            events.extend(abilities.diff(abilities_new));
        }

        if let (Some(players), Some(players_new)) = (self.players.as_ref(), new.players.as_ref()) {
            events.extend(players.diff(players_new));
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::abilities::{AbilityID, GameAbilities};
    use crate::components::players::{GamePlayers, PlayerActivity, PlayerID, PlayerInformation};
    use crate::components::team::Team;
    use crate::components::{DotaGameRulesState, Provider};
    use crate::event::{Ability as AbilityEvent, Map as MapEvent, Player as PlayerEvent};
    use std::collections::HashMap;

    // === Helpers ===

    fn make_ability(level: u8, can_cast: bool, cooldown: u16, active: bool) -> abilities::Ability {
        abilities::Ability {
            name: "test_ability".to_string(),
            level,
            can_cast,
            passive: false,
            ability_active: active,
            cooldown,
            ultimate: false,
        }
    }

    fn make_player_info(kills: u16, deaths: u16, assists: u16, streak: u16) -> PlayerInformation {
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

    fn make_map(daytime: bool, nightstalker_night: bool) -> components::Map {
        components::Map {
            name: "start".to_string(),
            match_id: "12345".to_string(),
            game_time: 600,
            clock_time: 600,
            daytime,
            nightstalker_night,
            game_state: DotaGameRulesState::InProgress,
            paused: false,
            win_team: Team::None,
            custom_game_name: "".to_string(),
            ward_purchase_cooldown: None,
        }
    }

    fn make_game_state(
        map: Option<components::Map>,
        players: Option<GamePlayers>,
        abilities: Option<GameAbilities>,
    ) -> GameState {
        GameState {
            provider: Provider {
                name: "Dota 2".to_string(),
                app_id: 570,
                version: 47,
                timestamp: 0,
            },
            buildings: None,
            map,
            players,
            heroes: None,
            abilities,
            items: None,
            draft: None,
            wearables: None,
            auth: None,
        }
    }

    // === Ability Diff Tests ===

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
        assert_eq!(events, vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp(2))]);
    }

    #[test]
    fn test_ability_level_up_multiple() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(4, true, 0, true);
        let events = prev.diff(&cur);
        assert_eq!(events, vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp(4))]);
    }

    #[test]
    fn test_ability_went_on_cooldown() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(1, false, 12, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::WentOnCooldown(12))]
        );
    }

    #[test]
    fn test_ability_went_off_cooldown() {
        let prev = make_ability(1, false, 5, true);
        let cur = make_ability(1, true, 0, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::WentOffCooldown)]
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
            vec![GameEvent::AbilityEvent(AbilityEvent::Activated)]
        );
    }

    #[test]
    fn test_ability_deactivated() {
        let prev = make_ability(1, true, 0, true);
        let cur = make_ability(1, true, 0, false);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::AbilityEvent(AbilityEvent::Deactivated)]
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
                GameEvent::AbilityEvent(AbilityEvent::LevelledUp(2)),
                GameEvent::AbilityEvent(AbilityEvent::WentOnCooldown(8)),
            ]
        );
    }

    // === GameAbilities Diff Tests (Playing) ===

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
        assert_eq!(events, vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp(2))]);
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

    // === GameAbilities Diff Tests (Spectating) ===

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
        assert_eq!(events, vec![GameEvent::AbilityEvent(AbilityEvent::LevelledUp(2))]);
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
        let radiant_cur: HashMap<PlayerID, HashMap<AbilityID, abilities::Ability>> = HashMap::new();

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

    // === PlayerInformation Diff Tests ===

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
            vec![GameEvent::PlayerEvent(PlayerEvent::Died(2))]
        );
    }

    #[test]
    fn test_player_info_assisted() {
        let prev = make_player_info(0, 0, 0, 0);
        let cur = make_player_info(0, 0, 1, 0);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::PlayerEvent(PlayerEvent::Assisted(1))]
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
                GameEvent::PlayerEvent(PlayerEvent::SecuredKill { kills: 3, streak: 1 }),
                GameEvent::PlayerEvent(PlayerEvent::Died(2)),
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
                kills: 3,
                streak: 3
            })]
        );
    }

    // === GamePlayers Diff Tests ===

    #[test]
    fn test_game_players_playing_kill() {
        let prev = GamePlayers::Playing(make_player_info(0, 0, 0, 0));
        let cur = GamePlayers::Playing(make_player_info(1, 0, 0, 1));
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
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
        assert!(events.iter().all(|e| matches!(e, GameEvent::PlayerEvent(PlayerEvent::Died(1)))));
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

    // === Map Diff Tests ===

    #[test]
    fn test_map_no_change_day() {
        let map = make_map(true, false);
        let events = map.diff(&map.clone());
        assert!(events.is_empty());
    }

    #[test]
    fn test_map_no_change_night() {
        let map = make_map(false, false);
        let events = map.diff(&map.clone());
        assert!(events.is_empty());
    }

    #[test]
    fn test_map_started_night() {
        let prev = make_map(true, false);
        let cur = make_map(false, false);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::MapEvent(MapEvent::StartedNight {
                nightstalker: false
            })]
        );
    }

    #[test]
    fn test_map_started_night_nightstalker() {
        let prev = make_map(true, false);
        let cur = make_map(false, true);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::MapEvent(MapEvent::StartedNight {
                nightstalker: true
            })]
        );
    }

    #[test]
    fn test_map_started_day() {
        let prev = make_map(false, false);
        let cur = make_map(true, false);
        let events = prev.diff(&cur);
        assert_eq!(events, vec![GameEvent::MapEvent(MapEvent::StartedDay)]);
    }

    // === GameState Diff Tests ===

    #[test]
    fn test_game_state_all_none_no_events() {
        let prev = make_game_state(None, None, None);
        let cur = make_game_state(None, None, None);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_game_state_map_day_to_night() {
        let prev = make_game_state(Some(make_map(true, false)), None, None);
        let cur = make_game_state(Some(make_map(false, false)), None, None);
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![GameEvent::MapEvent(MapEvent::StartedNight {
                nightstalker: false
            })]
        );
    }

    #[test]
    fn test_game_state_none_to_some_map_no_events() {
        let prev = make_game_state(None, None, None);
        let cur = make_game_state(Some(make_map(true, false)), None, None);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_game_state_some_to_none_map_no_events() {
        let prev = make_game_state(Some(make_map(true, false)), None, None);
        let cur = make_game_state(None, None, None);
        let events = prev.diff(&cur);
        assert!(events.is_empty());
    }

    #[test]
    fn test_game_state_combined_events() {
        let prev = make_game_state(
            Some(make_map(true, false)),
            Some(GamePlayers::Playing(make_player_info(0, 0, 0, 0))),
            None,
        );
        let cur = make_game_state(
            Some(make_map(false, false)),
            Some(GamePlayers::Playing(make_player_info(1, 0, 0, 1))),
            None,
        );
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![
                GameEvent::MapEvent(MapEvent::StartedNight { nightstalker: false }),
                GameEvent::PlayerEvent(PlayerEvent::SecuredKill { kills: 1, streak: 1 }),
            ]
        );
    }

    #[test]
    fn test_game_state_player_and_ability_events() {
        let mut prev_abilities = HashMap::new();
        prev_abilities.insert(AbilityID(0), make_ability(1, true, 0, true));

        let mut cur_abilities = HashMap::new();
        cur_abilities.insert(AbilityID(0), make_ability(2, false, 10, true));

        let prev = make_game_state(
            None,
            Some(GamePlayers::Playing(make_player_info(0, 0, 0, 0))),
            Some(GameAbilities::Playing(prev_abilities)),
        );
        let cur = make_game_state(
            None,
            Some(GamePlayers::Playing(make_player_info(0, 1, 0, 0))),
            Some(GameAbilities::Playing(cur_abilities)),
        );
        let events = prev.diff(&cur);
        assert_eq!(
            events,
            vec![
                GameEvent::AbilityEvent(AbilityEvent::LevelledUp(2)),
                GameEvent::AbilityEvent(AbilityEvent::WentOnCooldown(10)),
                GameEvent::PlayerEvent(PlayerEvent::Died(1)),
            ]
        );
    }

    #[test]
    fn test_game_state_identical_no_events() {
        let abilities = {
            let mut m = HashMap::new();
            m.insert(AbilityID(0), make_ability(3, true, 0, true));
            m
        };

        let state = make_game_state(
            Some(make_map(true, false)),
            Some(GamePlayers::Playing(make_player_info(5, 2, 3, 1))),
            Some(GameAbilities::Playing(abilities.clone())),
        );
        let events = state.diff(&state.clone());
        assert!(events.is_empty());
    }
}

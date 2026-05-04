/// Provides a handler for [`GameState`] diffs
use async_trait::async_trait;

use crate::components::GameState;
use crate::diff::Diffable;
use crate::event::GameEvent;
use crate::{HandlerResult, MutHandler};

/// Handler that sends events to a consumer
///
/// The first [`GameState`] will not generate any events as there is no previous state
/// to diff against. Events will be sent to the consumer starting on the second tick.
pub struct DiffHandler<F> {
    state: Option<GameState>,
    consumer: F,
}

impl<F> DiffHandler<F> {
    pub fn new(consumer: F) -> Self {
        Self {
            state: None,
            consumer,
        }
    }
}

#[async_trait]
impl<F, Fut> MutHandler for DiffHandler<F>
where
    F: Fn(Vec<GameEvent>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), anyhow::Error>> + Send,
{
    async fn handle(&mut self, event: bytes::Bytes) -> HandlerResult {
        let current: GameState = serde_json::from_slice(&event)?;

        if let Some(state) = self.state.as_ref() {
            let events = state.diff(&current);

            (self.consumer)(events).await?;
        }

        self.state = Some(current);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::players::{GamePlayers, PlayerActivity, PlayerInformation};
    use crate::components::team::Team;
    use crate::components::{DotaGameRulesState, Provider};
    use crate::event::{Map as MapEvent, Player as PlayerEvent};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn make_game_state_json(daytime: bool, kills: u16, deaths: u16) -> Vec<u8> {
        let daytime_str = if daytime { "true" } else { "false" };
        format!(
            r#"{{
                "provider": {{ "name": "Dota 2", "appid": 570, "version": 47, "timestamp": 0 }},
                "map": {{
                    "name": "start",
                    "matchid": "12345",
                    "game_time": 600,
                    "clock_time": 600,
                    "daytime": {daytime_str},
                    "nightstalker_night": false,
                    "game_state": "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS",
                    "paused": false,
                    "win_team": "none",
                    "customgamename": ""
                }},
                "player": {{
                    "steamid": "76561198000000000",
                    "name": "TestPlayer",
                    "activity": "playing",
                    "kills": {kills},
                    "deaths": {deaths},
                    "assists": 0,
                    "last_hits": 0,
                    "denies": 0,
                    "kill_streak": 0,
                    "kill_list": {{}},
                    "commands_issued": 0,
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
                }}
            }}"#
        )
        .into_bytes()
    }

    fn make_initial_game_state(daytime: bool, kills: u16, deaths: u16) -> GameState {
        GameState {
            provider: Provider {
                name: "Dota 2".to_string(),
                app_id: 570,
                version: 47,
                timestamp: 0,
            },
            buildings: None,
            map: Some(crate::components::Map {
                name: "start".to_string(),
                match_id: "12345".to_string(),
                game_time: 600,
                clock_time: 600,
                daytime,
                nightstalker_night: false,
                game_state: DotaGameRulesState::InProgress,
                paused: false,
                win_team: Team::None,
                custom_game_name: "".to_string(),
                ward_purchase_cooldown: None,
            }),
            players: Some(GamePlayers::Playing(PlayerInformation {
                steamid: "76561198000000000".to_string(),
                name: "TestPlayer".to_string(),
                activity: PlayerActivity::Playing,
                kills,
                deaths,
                assists: 0,
                last_hits: 0,
                denies: 0,
                kill_streak: 0,
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
            })),
            heroes: None,
            abilities: None,
            items: None,
            draft: None,
            wearables: None,
            auth: None,
        }
    }

    #[tokio::test]
    async fn test_diff_handler_produces_map_event() {
        let captured: Arc<Mutex<Vec<Vec<GameEvent>>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        let initial = make_initial_game_state(true, 0, 0);
        let mut handler = DiffHandler {
            state: Some(initial),
            consumer: move |events: Vec<GameEvent>| {
                let captured = captured_clone.clone();
                async move {
                    captured.lock().await.push(events);
                    Ok(())
                }
            },
        };

        // Send a state where it's now nighttime
        let night_json = make_game_state_json(false, 0, 0);
        handler
            .handle(bytes::Bytes::from(night_json))
            .await
            .expect("handler should succeed");

        let results = captured.lock().await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            vec![GameEvent::MapEvent(MapEvent::StartedNight {
                nightstalker: false
            })]
        );
    }

    #[tokio::test]
    async fn test_diff_handler_produces_player_event() {
        let captured: Arc<Mutex<Vec<Vec<GameEvent>>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        let initial = make_initial_game_state(true, 0, 0);
        let mut handler = DiffHandler {
            state: Some(initial),
            consumer: move |events: Vec<GameEvent>| {
                let captured = captured_clone.clone();
                async move {
                    captured.lock().await.push(events);
                    Ok(())
                }
            },
        };

        // Send a state where player got a kill
        let kill_json = make_game_state_json(true, 1, 0);
        handler
            .handle(bytes::Bytes::from(kill_json))
            .await
            .expect("handler should succeed");

        let results = captured.lock().await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            vec![GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                name: "TestPlayer".to_owned(),
                kills: 1,
                streak: 0
            })]
        );
    }

    #[tokio::test]
    async fn test_diff_handler_no_events_when_identical() {
        let captured: Arc<Mutex<Vec<Vec<GameEvent>>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        let initial = make_initial_game_state(true, 0, 0);
        let mut handler = DiffHandler {
            state: Some(initial),
            consumer: move |events: Vec<GameEvent>| {
                let captured = captured_clone.clone();
                async move {
                    captured.lock().await.push(events);
                    Ok(())
                }
            },
        };

        // Send same state as initial
        let same_json = make_game_state_json(true, 0, 0);
        handler
            .handle(bytes::Bytes::from(same_json))
            .await
            .expect("handler should succeed");

        let results = captured.lock().await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_empty());
    }

    #[tokio::test]
    async fn test_diff_handler_updates_state_between_calls() {
        let captured: Arc<Mutex<Vec<Vec<GameEvent>>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        let initial = make_initial_game_state(true, 0, 0);
        let mut handler = DiffHandler {
            state: Some(initial),
            consumer: move |events: Vec<GameEvent>| {
                let captured = captured_clone.clone();
                async move {
                    captured.lock().await.push(events);
                    Ok(())
                }
            },
        };

        // First call: 0 kills -> 1 kill
        let json1 = make_game_state_json(true, 1, 0);
        handler
            .handle(bytes::Bytes::from(json1))
            .await
            .expect("handler should succeed");

        // Second call: 1 kill -> 2 kills (should diff against updated state, not initial)
        let json2 = make_game_state_json(true, 2, 0);
        handler
            .handle(bytes::Bytes::from(json2))
            .await
            .expect("handler should succeed");

        let results = captured.lock().await;
        assert_eq!(results.len(), 2);

        // First call: detected a kill (0 -> 1)
        assert_eq!(
            results[0],
            vec![GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                name: "TestPlayer".to_owned(),
                kills: 1,
                streak: 0
            })]
        );

        // Second call: detected another kill (1 -> 2), proving state was updated
        assert_eq!(
            results[1],
            vec![GameEvent::PlayerEvent(PlayerEvent::SecuredKill {
                name: "TestPlayer".to_owned(),
                kills: 2,
                streak: 0
            })]
        );
    }

    #[tokio::test]
    async fn test_diff_handler_invalid_json_returns_error() {
        let mut handler = DiffHandler {
            state: Some(make_initial_game_state(true, 0, 0)),
            consumer: |_events: Vec<GameEvent>| async move { Ok(()) },
        };

        let invalid_json = bytes::Bytes::from_static(b"not valid json{{{");
        let result = handler.handle(invalid_json).await;
        assert!(result.is_err());
    }
}

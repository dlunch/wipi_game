use alloc::string::String;
use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::data::{Direction, Map, Tile};
use crate::game::{ExploreAction, ExploreUiState, GameData, GameState, PlayerState};

#[derive(Debug, Clone, Copy)]
pub enum ExploreIntent {
    MoveDirection(KeyCode),
    TryNpcInteract,
    UseAction(ExploreAction),
    Pause,
    BackToMenu,
}

pub enum ExploreEvent {
    None,
    MoveDirection(KeyCode),
    TryNpcInteract { facing: Direction },
    UseAction(ExploreAction),
    EnterPauseMenu,
    EnterMenu,
}

impl ExploreIntent {
    pub fn intent_for_key(ui: &ExploreUiState, key: KeyCode) -> Vec<ExploreIntent> {
        let mut intents = Vec::new();
        match key {
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                intents.push(ExploreIntent::MoveDirection(key));
            }
            KeyCode::Ok => {
                intents.push(ExploreIntent::TryNpcInteract);
            }
            KeyCode::Key0 => intents.push(ExploreIntent::Pause),
            KeyCode::Back => intents.push(ExploreIntent::BackToMenu),
            _ => {}
        }

        if let Some(action) = ui.action_for_key(key) {
            intents.push(ExploreIntent::UseAction(action));
        }

        intents
    }
}

pub fn reduce(
    state: &GameState,
    player: &PlayerState,
    data: &GameData,
    intent: ExploreIntent,
) -> ExploreEvent {
    let is_peaceful = data
        .find_map(&player.current_map_id)
        .is_some_and(|m| m.peaceful);

    match intent {
        ExploreIntent::MoveDirection(key) => ExploreEvent::MoveDirection(key),
        ExploreIntent::TryNpcInteract => ExploreEvent::TryNpcInteract {
            facing: player.facing,
        },
        ExploreIntent::UseAction(action) if !is_peaceful => {
            if matches!(*state, GameState::Dialog) {
                ExploreEvent::None
            } else {
                ExploreEvent::UseAction(action)
            }
        }
        ExploreIntent::UseAction(_) => ExploreEvent::None,
        ExploreIntent::Pause => ExploreEvent::EnterPauseMenu,
        ExploreIntent::BackToMenu => ExploreEvent::EnterMenu,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileEvent {
    Treasure,
    MapExit(String),
    DungeonEntrance(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileApplyEvent {
    None,
    MapChanged,
}

fn check_tile_event(map: &Map, player: &PlayerState) -> Option<TileEvent> {
    let tile = map.get_tile(player.x, player.y);

    match tile {
        Tile::Treasure => Some(TileEvent::Treasure),
        Tile::Exit => {
            for (ex, ey, target) in &map.exits {
                if *ex == player.x && *ey == player.y {
                    return Some(TileEvent::MapExit(target.clone()));
                }
            }
            None
        }
        Tile::Dungeon => {
            for (dx, dy, target) in &map.dungeons {
                if *dx == player.x && *dy == player.y {
                    return Some(TileEvent::DungeonEntrance(target.clone()));
                }
            }
            None
        }
        _ => None,
    }
}

pub fn reduce_tile_event(player: &PlayerState, data: &GameData) -> Option<TileEvent> {
    data.find_map(&player.current_map_id)
        .and_then(|map| check_tile_event(map, player))
}

pub fn apply_tile_event(
    player: &mut PlayerState,
    data: &GameData,
    event: TileEvent,
) -> TileApplyEvent {
    match event {
        TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
            if !target.is_empty() && change_map(player, data, &target) {
                TileApplyEvent::MapChanged
            } else {
                TileApplyEvent::None
            }
        }
        TileEvent::Treasure => {
            let map_id = player.current_map_id.clone();
            if !player.is_treasure_opened(&map_id, player.x, player.y) {
                if let Some(potion) = data.find_item("potion").cloned() {
                    player.inventory.push(potion);
                }
                player.opened_treasures.push((map_id, player.x, player.y));
            }
            TileApplyEvent::None
        }
    }
}

fn change_map(player: &mut PlayerState, data: &GameData, target_id: &str) -> bool {
    let Some(map) = data.find_map(target_id) else {
        return false;
    };

    let (x, y) = map.find_player_start().unwrap_or((player.x, player.y));
    player.current_map_id = map.id.clone();
    player.x = x;
    player.y = y;
    true
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use wipi::event::KeyCode;

    use super::*;
    use crate::data::{Direction, Item, ItemKind, Map, Tile};

    fn make_test_map(
        id: &str,
        width: usize,
        height: usize,
        tiles: Vec<Tile>,
        exits: Vec<(usize, usize, String)>,
        dungeons: Vec<(usize, usize, String)>,
        encounters: Vec<(String, i32)>,
        peaceful: bool,
    ) -> Map {
        Map {
            id: id.into(),
            name: id.into(),
            width,
            height,
            tiles,
            encounters,
            exits,
            dungeons,
            npcs: Vec::new(),
            peaceful,
        }
    }

    fn make_game_data() -> GameData {
        let mut data = GameData::default();

        data.items.push(Item {
            id: "potion".into(),
            name: "Potion".into(),
            kind: ItemKind::Consumable,
            param1: 30,
            param2: 0,
            param3: 0,
            price: 50,
        });

        data.maps.push(make_test_map(
            "field",
            3,
            3,
            vec![
                Tile::PlayerStart,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Treasure,
                Tile::Exit,
                Tile::Floor,
                Tile::Dungeon,
                Tile::Floor,
            ],
            vec![(2, 1, "town".into())],
            vec![(1, 2, "cave".into())],
            Vec::new(),
            false,
        ));

        data.maps.push(make_test_map(
            "town",
            3,
            3,
            vec![
                Tile::PlayerStart,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        ));

        data.maps.push(make_test_map(
            "cave",
            3,
            3,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::PlayerStart,
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        ));

        data.maps.push(make_test_map(
            "safe_zone",
            2,
            2,
            vec![Tile::PlayerStart, Tile::Floor, Tile::Floor, Tile::Floor],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        ));

        data
    }

    fn make_player(map_id: &str, x: usize, y: usize) -> PlayerState {
        let mut player = PlayerState::new(String::from("Tester"), map_id);
        player.x = x;
        player.y = y;
        player
    }

    #[test]
    fn intent_for_key_direction_keys_map_to_move_direction() {
        let ui = ExploreUiState::default();
        for key in [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right] {
            let intents = ExploreIntent::intent_for_key(&ui, key);
            assert_eq!(intents.len(), 1);
            assert!(matches!(intents.as_slice(), [ExploreIntent::MoveDirection(k)] if *k == key));
        }
    }

    #[test]
    fn intent_for_key_ok_returns_interact_then_attack() {
        let ui = ExploreUiState::default();
        let intents = ExploreIntent::intent_for_key(&ui, KeyCode::Ok);
        assert!(matches!(
            intents.as_slice(),
            [
                ExploreIntent::TryNpcInteract,
                ExploreIntent::UseAction(ExploreAction::BasicAttack)
            ]
        ));
    }

    #[test]
    fn intent_for_key_skill_keys_map_to_skills() {
        let ui = ExploreUiState::default();
        assert!(matches!(
            ExploreIntent::intent_for_key(&ui, KeyCode::Key1).as_slice(),
            [ExploreIntent::UseAction(ExploreAction::Fireball)]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(&ui, KeyCode::Key2).as_slice(),
            [ExploreIntent::UseAction(ExploreAction::Heal)]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(&ui, KeyCode::Key3).as_slice(),
            [ExploreIntent::UseAction(ExploreAction::SpinAttack)]
        ));
    }

    #[test]
    fn intent_for_key_pause_and_back_keys_map_to_menu_intents() {
        let ui = ExploreUiState::default();
        assert!(matches!(
            ExploreIntent::intent_for_key(&ui, KeyCode::Key0).as_slice(),
            [ExploreIntent::Pause]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(&ui, KeyCode::Back).as_slice(),
            [ExploreIntent::BackToMenu]
        ));
    }

    #[test]
    fn reduce_tile_event_treasure_returns_treasure_event() {
        let data = make_game_data();
        let player = make_player("field", 1, 1);

        let event = reduce_tile_event(&player, &data);

        assert!(matches!(event, Some(TileEvent::Treasure)));
    }

    #[test]
    fn apply_tile_event_treasure_adds_potion_and_records_opened() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 1);

        let apply_event = apply_tile_event(&mut player, &data, TileEvent::Treasure);

        assert!(matches!(apply_event, TileApplyEvent::None));
        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.inventory[0].id, "potion");
        assert_eq!(player.opened_treasures.len(), 1);
        assert!(player.is_treasure_opened("field", 1, 1));
    }

    #[test]
    fn apply_tile_event_already_opened_treasure_has_no_duplicate_rewards() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 1);

        apply_tile_event(&mut player, &data, TileEvent::Treasure);
        apply_tile_event(&mut player, &data, TileEvent::Treasure);

        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.opened_treasures.len(), 1);
    }

    #[test]
    fn reduce_tile_event_exit_tile_returns_map_exit_event() {
        let data = make_game_data();
        let player = make_player("field", 2, 1);

        let event = reduce_tile_event(&player, &data);

        assert!(matches!(event, Some(TileEvent::MapExit(target)) if target == "town"));
    }

    #[test]
    fn apply_tile_event_map_exit_changes_map_when_exit_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 2, 1);

        let apply_event = apply_tile_event(&mut player, &data, TileEvent::MapExit("town".into()));

        assert!(matches!(apply_event, TileApplyEvent::MapChanged));
        assert_eq!(player.current_map_id, "town");
        assert_eq!(player.x, 0);
        assert_eq!(player.y, 0);
    }

    #[test]
    fn reduce_tile_event_dungeon_tile_returns_dungeon_event() {
        let data = make_game_data();
        let player = make_player("field", 1, 2);

        let event = reduce_tile_event(&player, &data);

        assert!(matches!(event, Some(TileEvent::DungeonEntrance(target)) if target == "cave"));
    }

    #[test]
    fn apply_tile_event_dungeon_tile_changes_map_when_dungeon_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 2);

        let apply_event = apply_tile_event(
            &mut player,
            &data,
            TileEvent::DungeonEntrance("cave".into()),
        );

        assert!(matches!(apply_event, TileApplyEvent::MapChanged));
        assert_eq!(player.current_map_id, "cave");
        assert_eq!(player.x, 2);
        assert_eq!(player.y, 2);
    }

    #[test]
    fn reduce_tile_event_floor_tile_returns_none() {
        let data = make_game_data();
        let player = make_player("field", 1, 0);

        let event = reduce_tile_event(&player, &data);

        assert!(event.is_none());
    }

    #[test]
    fn apply_tile_event_map_exit_with_missing_target_does_nothing() {
        let data = make_game_data();
        let mut player = make_player("field", 2, 1);

        let apply_event = apply_tile_event(
            &mut player,
            &data,
            TileEvent::MapExit(String::from("missing")),
        );

        assert!(matches!(apply_event, TileApplyEvent::None));
        assert_eq!(player.current_map_id, "field");
    }

    #[test]
    fn reduce_pause_switches_to_pause_menu_state() {
        let data = make_game_data();
        let state = GameState::Explore;
        let player = make_player("field", 0, 0);

        let event = reduce(&state, &player, &data, ExploreIntent::Pause);

        assert!(matches!(event, ExploreEvent::EnterPauseMenu));
    }

    #[test]
    fn reduce_attack_has_no_effect_in_peaceful_zone() {
        let data = make_game_data();
        let state = GameState::Explore;
        let mut player = make_player("safe_zone", 0, 0);
        player.facing = Direction::Right;
        let event = reduce(
            &state,
            &player,
            &data,
            ExploreIntent::UseAction(ExploreAction::BasicAttack),
        );

        assert!(matches!(event, ExploreEvent::None));
    }
}

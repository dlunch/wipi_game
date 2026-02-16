use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::data::Direction;
use crate::game::ExploreAction;
#[cfg(test)]
use crate::game::{GameData, PlayerState};

#[derive(Debug, Clone, Copy)]
pub enum ExploreIntent {
    MoveDirection(Direction),
    TryNpcInteract {
        facing: Direction,
        fallback_action: Option<ExploreAction>,
    },
    UseAction(ExploreAction),
    Pause,
    BackToMenu,
}

pub enum ExploreEvent {
    None,
    MoveDirection(Direction),
    TryNpcInteract {
        facing: Direction,
        fallback_action: Option<ExploreAction>,
    },
    UseAction(ExploreAction),
    EnterPauseMenu,
    EnterMenu,
}

impl ExploreIntent {
    pub fn intent_for_key(
        key: KeyCode,
        facing: Direction,
        ok_action: ExploreAction,
        key_actions: [Option<ExploreAction>; 3],
    ) -> Vec<ExploreIntent> {
        let mut intents = Vec::new();
        match key {
            KeyCode::Up => intents.push(ExploreIntent::MoveDirection(Direction::Up)),
            KeyCode::Down => intents.push(ExploreIntent::MoveDirection(Direction::Down)),
            KeyCode::Left => intents.push(ExploreIntent::MoveDirection(Direction::Left)),
            KeyCode::Right => intents.push(ExploreIntent::MoveDirection(Direction::Right)),
            KeyCode::Ok => {
                intents.push(ExploreIntent::TryNpcInteract {
                    facing,
                    fallback_action: Some(ok_action),
                });
            }
            KeyCode::Key1 => {
                if let Some(action) = key_actions[0] {
                    intents.push(ExploreIntent::UseAction(action));
                }
            }
            KeyCode::Key2 => {
                if let Some(action) = key_actions[1] {
                    intents.push(ExploreIntent::UseAction(action));
                }
            }
            KeyCode::Key3 => {
                if let Some(action) = key_actions[2] {
                    intents.push(ExploreIntent::UseAction(action));
                }
            }
            KeyCode::Key0 => intents.push(ExploreIntent::Pause),
            KeyCode::Back => intents.push(ExploreIntent::BackToMenu),
            _ => {}
        }

        intents
    }
}

pub fn reduce(is_peaceful: bool, intent: ExploreIntent) -> ExploreEvent {
    match intent {
        ExploreIntent::MoveDirection(direction) => ExploreEvent::MoveDirection(direction),
        ExploreIntent::TryNpcInteract {
            facing,
            fallback_action,
        } => ExploreEvent::TryNpcInteract {
            facing,
            fallback_action,
        },
        ExploreIntent::UseAction(action) if !is_peaceful => ExploreEvent::UseAction(action),
        ExploreIntent::UseAction(_) => ExploreEvent::None,
        ExploreIntent::Pause => ExploreEvent::EnterPauseMenu,
        ExploreIntent::BackToMenu => ExploreEvent::EnterMenu,
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use wipi::event::KeyCode;

    use super::*;
    use crate::data::{Direction, Item, ItemKind, Map, Tile};
    use crate::game::TileEvent;
    use crate::game::state::TileApplyEvent;

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
        let key_actions = [
            Some(ExploreAction::Fireball),
            Some(ExploreAction::Heal),
            Some(ExploreAction::SpinAttack),
        ];
        for key in [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right] {
            let intents = ExploreIntent::intent_for_key(
                key,
                Direction::Down,
                ExploreAction::BasicAttack,
                key_actions,
            );
            assert_eq!(intents.len(), 1);
            assert!(matches!(
                intents.as_slice(),
                [ExploreIntent::MoveDirection(Direction::Up)]
                    | [ExploreIntent::MoveDirection(Direction::Down)]
                    | [ExploreIntent::MoveDirection(Direction::Left)]
                    | [ExploreIntent::MoveDirection(Direction::Right)]
            ));
        }
    }

    #[test]
    fn intent_for_key_ok_returns_interact_with_fallback_action() {
        let intents = ExploreIntent::intent_for_key(
            KeyCode::Ok,
            Direction::Up,
            ExploreAction::BasicAttack,
            [
                Some(ExploreAction::Fireball),
                Some(ExploreAction::Heal),
                Some(ExploreAction::SpinAttack),
            ],
        );
        assert!(matches!(
            intents.as_slice(),
            [ExploreIntent::TryNpcInteract {
                facing: Direction::Up,
                fallback_action: Some(ExploreAction::BasicAttack)
            }]
        ));
    }

    #[test]
    fn intent_for_key_skill_keys_map_to_skills() {
        let key_actions = [
            Some(ExploreAction::Fireball),
            Some(ExploreAction::Heal),
            Some(ExploreAction::SpinAttack),
        ];
        assert!(matches!(
            ExploreIntent::intent_for_key(
                KeyCode::Key1,
                Direction::Down,
                ExploreAction::BasicAttack,
                key_actions
            )
            .as_slice(),
            [ExploreIntent::UseAction(ExploreAction::Fireball)]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(
                KeyCode::Key2,
                Direction::Down,
                ExploreAction::BasicAttack,
                key_actions
            )
            .as_slice(),
            [ExploreIntent::UseAction(ExploreAction::Heal)]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(
                KeyCode::Key3,
                Direction::Down,
                ExploreAction::BasicAttack,
                key_actions
            )
            .as_slice(),
            [ExploreIntent::UseAction(ExploreAction::SpinAttack)]
        ));
    }

    #[test]
    fn intent_for_key_pause_and_back_keys_map_to_menu_intents() {
        let key_actions = [
            Some(ExploreAction::Fireball),
            Some(ExploreAction::Heal),
            Some(ExploreAction::SpinAttack),
        ];
        assert!(matches!(
            ExploreIntent::intent_for_key(
                KeyCode::Key0,
                Direction::Down,
                ExploreAction::BasicAttack,
                key_actions
            )
            .as_slice(),
            [ExploreIntent::Pause]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(
                KeyCode::Back,
                Direction::Down,
                ExploreAction::BasicAttack,
                key_actions
            )
            .as_slice(),
            [ExploreIntent::BackToMenu]
        ));
    }

    #[test]
    fn apply_tile_event_treasure_adds_potion_and_records_opened() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 1);

        let apply_event = player.apply_tile_event(&data, TileEvent::Treasure);

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

        player.apply_tile_event(&data, TileEvent::Treasure);
        player.apply_tile_event(&data, TileEvent::Treasure);

        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.opened_treasures.len(), 1);
    }

    #[test]
    fn apply_tile_event_map_exit_changes_map_when_exit_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 2, 1);

        let apply_event = player.apply_tile_event(&data, TileEvent::MapExit("town".into()));

        assert!(matches!(apply_event, TileApplyEvent::MapChanged));
        assert_eq!(player.current_map_id, "town");
        assert_eq!(player.x, 0);
        assert_eq!(player.y, 0);
    }

    #[test]
    fn apply_tile_event_dungeon_tile_changes_map_when_dungeon_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 2);

        let apply_event = player.apply_tile_event(&data, TileEvent::DungeonEntrance("cave".into()));

        assert!(matches!(apply_event, TileApplyEvent::MapChanged));
        assert_eq!(player.current_map_id, "cave");
        assert_eq!(player.x, 2);
        assert_eq!(player.y, 2);
    }

    #[test]
    fn apply_tile_event_map_exit_with_missing_target_does_nothing() {
        let data = make_game_data();
        let mut player = make_player("field", 2, 1);

        let apply_event =
            player.apply_tile_event(&data, TileEvent::MapExit(String::from("missing")));

        assert!(matches!(apply_event, TileApplyEvent::None));
        assert_eq!(player.current_map_id, "field");
    }

    #[test]
    fn reduce_pause_switches_to_pause_menu_state() {
        let event = reduce(false, ExploreIntent::Pause);

        assert!(matches!(event, ExploreEvent::EnterPauseMenu));
    }

    #[test]
    fn reduce_attack_has_no_effect_in_peaceful_zone() {
        let event = reduce(true, ExploreIntent::UseAction(ExploreAction::BasicAttack));

        assert!(matches!(event, ExploreEvent::None));
    }
}

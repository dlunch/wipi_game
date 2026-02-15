use wipi::event::KeyCode;

use crate::game::{GameState, InventoryState, MenuAction, PlayerState, save_game};

#[derive(Debug, Clone, Copy)]
pub enum MenuIntent {
    MoveUp,
    MoveDown,
    Select,
}

impl MenuIntent {
    pub fn intent_for_key(key: KeyCode) -> Option<MenuIntent> {
        match key {
            KeyCode::Up => Some(MenuIntent::MoveUp),
            KeyCode::Down => Some(MenuIntent::MoveDown),
            KeyCode::Ok => Some(MenuIntent::Select),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuEvent {
    None,
    Action(MenuAction),
}

#[derive(Debug, Clone, Copy)]
pub enum PauseMenuIntent {
    MoveUp,
    MoveDown,
    Select,
    Back,
}

impl PauseMenuIntent {
    pub fn intent_for_key(key: KeyCode) -> Option<PauseMenuIntent> {
        match key {
            KeyCode::Up => Some(PauseMenuIntent::MoveUp),
            KeyCode::Down => Some(PauseMenuIntent::MoveDown),
            KeyCode::Ok => Some(PauseMenuIntent::Select),
            KeyCode::Back | KeyCode::Key0 => Some(PauseMenuIntent::Back),
            _ => None,
        }
    }
}

pub fn reduce(state: &mut GameState, intent: MenuIntent) -> MenuEvent {
    let GameState::Menu(ref mut menu) = *state else {
        return MenuEvent::None;
    };

    match intent {
        MenuIntent::MoveUp => {
            if menu.selected > 0 {
                menu.selected -= 1;
            }
            MenuEvent::None
        }
        MenuIntent::MoveDown => {
            if menu.selected < menu.items.len() - 1 {
                menu.selected += 1;
            }
            MenuEvent::None
        }
        MenuIntent::Select => {
            let (_, action) = menu.items[menu.selected];
            MenuEvent::Action(action)
        }
    }
}

pub fn reduce_pause(
    state: &mut GameState,
    player: &PlayerState,
    inventory_state: &mut InventoryState,
    intent: PauseMenuIntent,
) {
    let GameState::PauseMenu(ref mut pause) = *state else {
        return;
    };

    match intent {
        PauseMenuIntent::MoveUp if pause.selected > 0 => pause.selected -= 1,
        PauseMenuIntent::MoveDown if pause.selected < pause.items.len() - 1 => pause.selected += 1,
        PauseMenuIntent::Select => match pause.selected {
            0 => {
                *inventory_state = InventoryState::default();
                *state = GameState::Inventory;
            }
            1 => *state = GameState::Stats,
            2 => *state = GameState::QuestLog,
            3 => {
                let _ = save_game(player);
                *state = GameState::Explore;
            }
            _ => {}
        },
        PauseMenuIntent::Back => *state = GameState::Explore,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::{MenuEvent, MenuIntent, PauseMenuIntent, reduce, reduce_pause};
    use crate::game::{
        GameState, InventoryState, MenuAction, MenuState, PauseMenuState, PlayerState,
    };

    fn menu_actions(menu: &MenuState) -> Vec<MenuAction> {
        menu.items.iter().map(|(_, action)| *action).collect()
    }

    fn pause_state_with_selected(selected: usize) -> GameState {
        let mut pause = PauseMenuState::new();
        pause.selected = selected;
        GameState::PauseMenu(pause)
    }

    #[test]
    fn menu_state_new_without_save_has_two_items() {
        let menu = MenuState::new(false);
        let actions = menu_actions(&menu);
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], MenuAction::NewGame));
        assert!(matches!(actions[1], MenuAction::Exit));
    }

    #[test]
    fn menu_state_new_with_save_has_three_items() {
        let menu = MenuState::new(true);
        let actions = menu_actions(&menu);
        assert_eq!(actions.len(), 3);
        assert!(matches!(actions[0], MenuAction::NewGame));
        assert!(matches!(actions[1], MenuAction::Continue));
        assert!(matches!(actions[2], MenuAction::Exit));
    }

    #[test]
    fn reduce_move_up_decrements_selected() {
        let mut menu = MenuState::new(true);
        menu.selected = 2;
        let mut state = GameState::Menu(menu);

        let event = reduce(&mut state, MenuIntent::MoveUp);

        assert!(matches!(event, MenuEvent::None));
        let GameState::Menu(menu) = state else {
            panic!("expected menu state");
        };
        assert_eq!(menu.selected, 1);
    }

    #[test]
    fn reduce_move_up_clamps_at_zero() {
        let mut state = GameState::Menu(MenuState::new(true));

        let event = reduce(&mut state, MenuIntent::MoveUp);

        assert!(matches!(event, MenuEvent::None));
        let GameState::Menu(menu) = state else {
            panic!("expected menu state");
        };
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn reduce_move_down_increments_selected() {
        let mut state = GameState::Menu(MenuState::new(false));

        let event = reduce(&mut state, MenuIntent::MoveDown);

        assert!(matches!(event, MenuEvent::None));
        let GameState::Menu(menu) = state else {
            panic!("expected menu state");
        };
        assert_eq!(menu.selected, 1);
    }

    #[test]
    fn reduce_move_down_clamps_at_last_item() {
        let mut menu = MenuState::new(false);
        menu.selected = menu.items.len() - 1;
        let mut state = GameState::Menu(menu);

        let event = reduce(&mut state, MenuIntent::MoveDown);

        assert!(matches!(event, MenuEvent::None));
        let GameState::Menu(menu) = state else {
            panic!("expected menu state");
        };
        assert_eq!(menu.selected, 1);
    }

    #[test]
    fn reduce_select_returns_action_for_selected_item() {
        let mut menu = MenuState::new(true);
        menu.selected = 1;
        let mut state = GameState::Menu(menu);

        let event = reduce(&mut state, MenuIntent::Select);

        assert!(matches!(event, MenuEvent::Action(MenuAction::Continue)));
    }

    #[test]
    fn reduce_non_menu_state_returns_none() {
        let mut state = GameState::Explore;

        let event = reduce(&mut state, MenuIntent::MoveDown);

        assert!(matches!(event, MenuEvent::None));
        assert!(matches!(state, GameState::Explore));
    }

    #[test]
    fn pause_menu_state_new_has_four_items_and_zero_selected() {
        let pause = PauseMenuState::new();
        assert_eq!(pause.items.len(), 4);
        assert_eq!(pause.selected, 0);
    }

    #[test]
    fn reduce_pause_move_up_and_down_navigate_and_clamp() {
        let mut state = pause_state_with_selected(0);
        let player = PlayerState::new(String::from("H"), "v");
        let mut inventory_state = InventoryState::default();

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::MoveUp,
        );
        assert!(matches!(state, GameState::PauseMenu(_)));
        if let GameState::PauseMenu(pause) = &state {
            assert_eq!(pause.selected, 0);
        }

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::MoveDown,
        );
        if let GameState::PauseMenu(pause) = &state {
            assert_eq!(pause.selected, 1);
        }

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::MoveDown,
        );
        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::MoveDown,
        );
        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::MoveDown,
        );
        if let GameState::PauseMenu(pause) = &state {
            assert_eq!(pause.selected, 3);
        }
    }

    #[test]
    fn reduce_pause_select_inventory_sets_inventory_and_resets_state() {
        let mut state = pause_state_with_selected(0);
        let player = PlayerState::new(String::from("H"), "v");
        let mut inventory_state = InventoryState {
            selected: 5,
            scroll: 3,
        };

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::Select,
        );

        assert!(matches!(state, GameState::Inventory));
        assert_eq!(inventory_state.selected, 0);
        assert_eq!(inventory_state.scroll, 0);
    }

    #[test]
    fn reduce_pause_select_stats_sets_stats_state() {
        let mut state = pause_state_with_selected(1);
        let player = PlayerState::new(String::from("H"), "v");
        let mut inventory_state = InventoryState::default();

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::Select,
        );

        assert!(matches!(state, GameState::Stats));
    }

    #[test]
    fn reduce_pause_select_quests_sets_quest_log_state() {
        let mut state = pause_state_with_selected(2);
        let player = PlayerState::new(String::from("H"), "v");
        let mut inventory_state = InventoryState::default();

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::Select,
        );

        assert!(matches!(state, GameState::QuestLog));
    }

    #[test]
    fn reduce_pause_select_save_sets_explore_state() {
        let mut state = pause_state_with_selected(3);
        let player = PlayerState::new(String::from("H"), "v");
        let mut inventory_state = InventoryState::default();

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::Select,
        );

        assert!(matches!(state, GameState::Explore));
    }

    #[test]
    fn reduce_pause_back_sets_explore_state() {
        let mut state = GameState::PauseMenu(PauseMenuState::new());
        let player = PlayerState::new(String::from("H"), "v");
        let mut inventory_state = InventoryState::default();

        reduce_pause(
            &mut state,
            &player,
            &mut inventory_state,
            PauseMenuIntent::Back,
        );

        assert!(matches!(state, GameState::Explore));
    }
}

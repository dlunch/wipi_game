use wipi::event::KeyCode;

use crate::game::{
    GameState, InventoryUiState, MenuAction, MenuUiState, PauseMenuUiState, PlayerState,
    ShopUiState, save_game,
};

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

pub fn reduce(state: &mut GameState, menu_ui: &mut MenuUiState, intent: MenuIntent) -> MenuEvent {
    let GameState::Menu = *state else {
        return MenuEvent::None;
    };

    match intent {
        MenuIntent::MoveUp => {
            if menu_ui.selected > 0 {
                menu_ui.selected -= 1;
            }
            MenuEvent::None
        }
        MenuIntent::MoveDown => {
            if menu_ui.selected < menu_ui.state.items.len() - 1 {
                menu_ui.selected += 1;
            }
            MenuEvent::None
        }
        MenuIntent::Select => {
            let (_, action) = menu_ui.state.items[menu_ui.selected];
            MenuEvent::Action(action)
        }
    }
}

pub fn reduce_pause(
    state: &mut GameState,
    player: &PlayerState,
    pause_ui: &mut PauseMenuUiState,
    inventory_ui: &mut InventoryUiState,
    shop_ui: &mut ShopUiState,
    intent: PauseMenuIntent,
) {
    let GameState::PauseMenu = *state else {
        return;
    };

    match intent {
        PauseMenuIntent::MoveUp if pause_ui.selected > 0 => pause_ui.selected -= 1,
        PauseMenuIntent::MoveDown if pause_ui.selected < pause_ui.state.items.len() - 1 => {
            pause_ui.selected += 1
        }
        PauseMenuIntent::Select => match pause_ui.selected {
            0 => {
                *inventory_ui = InventoryUiState::default();
                *state = GameState::Inventory;
            }
            1 => *state = GameState::Stats,
            2 => *state = GameState::QuestLog,
            3 => {
                let _ = save_game(player);
                *shop_ui = ShopUiState::default();
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

    use super::{MenuEvent, MenuIntent, PauseMenuIntent, reduce, reduce_pause};
    use crate::game::{
        GameState, InventoryUiState, MenuAction, MenuState, MenuUiState, PauseMenuUiState,
        PlayerState, ShopUiState,
    };

    #[test]
    fn reduce_move_and_select_uses_menu_ui_state() {
        let mut state = GameState::Menu;
        let mut ui = MenuUiState {
            state: MenuState::new(true),
            selected: 0,
        };

        let _ = reduce(&mut state, &mut ui, MenuIntent::MoveDown);
        let _ = reduce(&mut state, &mut ui, MenuIntent::MoveDown);
        let _ = reduce(&mut state, &mut ui, MenuIntent::MoveDown);
        assert_eq!(ui.selected, 2);

        let event = reduce(&mut state, &mut ui, MenuIntent::Select);
        assert!(matches!(event, MenuEvent::Action(MenuAction::Exit)));
    }

    #[test]
    fn reduce_non_menu_state_returns_none() {
        let mut state = GameState::Explore;
        let mut ui = MenuUiState::default();
        let event = reduce(&mut state, &mut ui, MenuIntent::MoveDown);
        assert!(matches!(event, MenuEvent::None));
        assert_eq!(ui.selected, 0);
    }

    #[test]
    fn reduce_pause_select_inventory_resets_inventory_ui() {
        let mut state = GameState::PauseMenu;
        let player = PlayerState::new(String::from("H"), "v");
        let mut pause_ui = PauseMenuUiState::default();
        let mut inventory_ui = InventoryUiState { selected: 5 };
        let mut shop_ui = ShopUiState::default();

        reduce_pause(
            &mut state,
            &player,
            &mut pause_ui,
            &mut inventory_ui,
            &mut shop_ui,
            PauseMenuIntent::Select,
        );

        assert!(matches!(state, GameState::Inventory));
        assert_eq!(inventory_ui.selected, 0);
    }

    #[test]
    fn reduce_pause_select_other_actions() {
        let player = PlayerState::new(String::from("H"), "v");
        let mut inventory_ui = InventoryUiState::default();
        let mut shop_ui = ShopUiState::default();
        let mut pause_ui = PauseMenuUiState::default();
        pause_ui.selected = 1;

        let mut state = GameState::PauseMenu;
        reduce_pause(
            &mut state,
            &player,
            &mut pause_ui,
            &mut inventory_ui,
            &mut shop_ui,
            PauseMenuIntent::Select,
        );
        assert!(matches!(state, GameState::Stats));

        state = GameState::PauseMenu;
        pause_ui.selected = 2;
        reduce_pause(
            &mut state,
            &player,
            &mut pause_ui,
            &mut inventory_ui,
            &mut shop_ui,
            PauseMenuIntent::Select,
        );
        assert!(matches!(state, GameState::QuestLog));

        state = GameState::PauseMenu;
        pause_ui.selected = 3;
        reduce_pause(
            &mut state,
            &player,
            &mut pause_ui,
            &mut inventory_ui,
            &mut shop_ui,
            PauseMenuIntent::Select,
        );
        assert!(matches!(state, GameState::Explore));
    }
}

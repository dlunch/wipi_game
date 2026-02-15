use wipi::event::KeyCode;

use crate::game::{GameState, MenuAction, MenuUiState, PauseMenuUiState};

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
    SetSelected(usize),
    Action(MenuAction),
}

pub enum PauseMenuEvent {
    None,
    SetSelected(usize),
    OpenInventory,
    OpenStats,
    OpenQuestLog,
    SaveAndReturnExplore,
    BackToExplore,
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

pub fn reduce(state: &GameState, menu_ui: &MenuUiState, intent: MenuIntent) -> MenuEvent {
    let GameState::Menu = *state else {
        return MenuEvent::None;
    };

    match intent {
        MenuIntent::MoveUp => {
            if menu_ui.selected > 0 {
                return MenuEvent::SetSelected(menu_ui.selected - 1);
            }
        }
        MenuIntent::MoveDown => {
            if menu_ui.selected < menu_ui.state.items.len() - 1 {
                return MenuEvent::SetSelected(menu_ui.selected + 1);
            }
        }
        MenuIntent::Select => {
            let (_, action) = menu_ui.state.items[menu_ui.selected];
            return MenuEvent::Action(action);
        }
    }

    MenuEvent::None
}

pub fn reduce_pause(
    state: &GameState,
    pause_ui: &PauseMenuUiState,
    intent: PauseMenuIntent,
) -> PauseMenuEvent {
    let GameState::PauseMenu = *state else {
        return PauseMenuEvent::None;
    };

    match intent {
        PauseMenuIntent::MoveUp if pause_ui.selected > 0 => {
            return PauseMenuEvent::SetSelected(pause_ui.selected - 1);
        }
        PauseMenuIntent::MoveDown if pause_ui.selected < pause_ui.state.items.len() - 1 => {
            return PauseMenuEvent::SetSelected(pause_ui.selected + 1);
        }
        PauseMenuIntent::Select => match pause_ui.selected {
            0 => return PauseMenuEvent::OpenInventory,
            1 => return PauseMenuEvent::OpenStats,
            2 => return PauseMenuEvent::OpenQuestLog,
            3 => return PauseMenuEvent::SaveAndReturnExplore,
            _ => {}
        },
        PauseMenuIntent::Back => return PauseMenuEvent::BackToExplore,
        _ => {}
    }

    PauseMenuEvent::None
}

#[cfg(test)]
mod tests {
    use super::{MenuEvent, MenuIntent, PauseMenuEvent, PauseMenuIntent, reduce, reduce_pause};
    use crate::game::{GameState, MenuAction, MenuState, MenuUiState, PauseMenuUiState};

    #[test]
    fn menu_reduce_returns_selection_and_action_events() {
        let state = GameState::Menu;
        let mut ui = MenuUiState {
            state: MenuState::new(true),
            selected: 0,
        };

        let event = reduce(&state, &ui, MenuIntent::MoveDown);
        assert!(matches!(event, MenuEvent::SetSelected(1)));
        ui.selected = 1;

        let event = reduce(&state, &ui, MenuIntent::MoveDown);
        assert!(matches!(event, MenuEvent::SetSelected(2)));
        ui.selected = 2;

        let event = reduce(&state, &ui, MenuIntent::Select);
        assert!(matches!(event, MenuEvent::Action(MenuAction::Exit)));
    }

    #[test]
    fn pause_reduce_returns_expected_events() {
        let state = GameState::PauseMenu;
        let mut ui = PauseMenuUiState::default();

        let event = reduce_pause(&state, &ui, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenInventory));

        ui.selected = 1;
        let event = reduce_pause(&state, &ui, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenStats));

        ui.selected = 2;
        let event = reduce_pause(&state, &ui, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenQuestLog));

        ui.selected = 3;
        let event = reduce_pause(&state, &ui, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::SaveAndReturnExplore));
    }
}

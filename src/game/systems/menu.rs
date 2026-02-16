use wipi::event::KeyCode;

use crate::game::MenuAction;

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

pub fn resolve(selected: usize, items: &[(&str, MenuAction)], intent: MenuIntent) -> MenuEvent {
    match intent {
        MenuIntent::MoveUp => {
            if selected > 0 {
                return MenuEvent::SetSelected(selected - 1);
            }
        }
        MenuIntent::MoveDown => {
            if selected + 1 < items.len() {
                return MenuEvent::SetSelected(selected + 1);
            }
        }
        MenuIntent::Select => {
            if let Some((_, action)) = items.get(selected).copied() {
                return MenuEvent::Action(action);
            }
        }
    }

    MenuEvent::None
}

pub fn resolve_pause(
    selected: usize,
    item_count: usize,
    intent: PauseMenuIntent,
) -> PauseMenuEvent {
    match intent {
        PauseMenuIntent::MoveUp if selected > 0 => {
            return PauseMenuEvent::SetSelected(selected - 1);
        }
        PauseMenuIntent::MoveDown if selected + 1 < item_count => {
            return PauseMenuEvent::SetSelected(selected + 1);
        }
        PauseMenuIntent::Select => match selected {
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
    use super::{MenuEvent, MenuIntent, PauseMenuEvent, PauseMenuIntent, resolve, resolve_pause};
    use crate::game::{MenuAction, MenuState};

    #[test]
    fn menu_reduce_returns_selection_and_action_events() {
        let items = MenuState::new(true).items;
        let mut selected = 0;

        let event = resolve(selected, &items, MenuIntent::MoveDown);
        assert!(matches!(event, MenuEvent::SetSelected(1)));
        selected = 1;

        let event = resolve(selected, &items, MenuIntent::MoveDown);
        assert!(matches!(event, MenuEvent::SetSelected(2)));
        selected = 2;

        let event = resolve(selected, &items, MenuIntent::Select);
        assert!(matches!(event, MenuEvent::Action(MenuAction::Exit)));
    }

    #[test]
    fn pause_reduce_returns_expected_events() {
        let mut selected = 0;
        let item_count = 4;

        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenInventory));

        selected = 1;
        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenStats));

        selected = 2;
        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenQuestLog));

        selected = 3;
        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::SaveAndReturnExplore));
    }
}

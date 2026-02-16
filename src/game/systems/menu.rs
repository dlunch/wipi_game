use crate::game::MenuAction;
use crate::game::selection::{step_down, step_up};

#[derive(Debug, Clone, Copy)]
pub enum MenuIntent {
    MoveUp,
    MoveDown,
    Select,
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

pub fn resolve(selected: usize, items: &[(&str, MenuAction)], intent: MenuIntent) -> MenuEvent {
    match intent {
        MenuIntent::MoveUp => {
            let next = step_up(selected);
            if next != selected {
                return MenuEvent::SetSelected(next);
            }
        }
        MenuIntent::MoveDown => {
            let next = step_down(selected, items.len());
            if next != selected {
                return MenuEvent::SetSelected(next);
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
        PauseMenuIntent::MoveUp => {
            let next = step_up(selected);
            if next != selected {
                return PauseMenuEvent::SetSelected(next);
            }
        }
        PauseMenuIntent::MoveDown => {
            let next = step_down(selected, item_count);
            if next != selected {
                return PauseMenuEvent::SetSelected(next);
            }
        }
        PauseMenuIntent::Select => match selected {
            0 => return PauseMenuEvent::OpenInventory,
            1 => return PauseMenuEvent::OpenStats,
            2 => return PauseMenuEvent::OpenQuestLog,
            3 => return PauseMenuEvent::SaveAndReturnExplore,
            _ => {}
        },
        PauseMenuIntent::Back => return PauseMenuEvent::BackToExplore,
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

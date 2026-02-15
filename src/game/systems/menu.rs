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

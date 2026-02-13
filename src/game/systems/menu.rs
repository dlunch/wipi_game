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
            menu.move_up();
            MenuEvent::None
        }
        MenuIntent::MoveDown => {
            menu.move_down();
            MenuEvent::None
        }
        MenuIntent::Select => {
            let action = if menu.has_save {
                match menu.selected {
                    0 => MenuAction::NewGame,
                    1 => MenuAction::Continue,
                    _ => MenuAction::Exit,
                }
            } else {
                match menu.selected {
                    0 => MenuAction::NewGame,
                    _ => MenuAction::Exit,
                }
            };
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
    let GameState::PauseMenu(ref mut selected) = *state else {
        return;
    };

    match intent {
        PauseMenuIntent::MoveUp if *selected > 0 => *selected -= 1,
        PauseMenuIntent::MoveDown if *selected < 3 => *selected += 1,
        PauseMenuIntent::Select => match *selected {
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

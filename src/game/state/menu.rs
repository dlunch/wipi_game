use wipi::event::KeyCode;

#[derive(Debug, Clone, Default)]
pub struct MenuState {
    pub selected: usize,
    pub has_save: bool,
}

impl MenuState {
    pub fn menu_count(&self) -> usize {
        if self.has_save { 3 } else { 2 }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < self.menu_count() - 1 {
            self.selected += 1;
        }
    }

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
pub enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy)]
pub enum MenuIntent {
    MoveUp,
    MoveDown,
    Select,
}

#[derive(Debug, Clone, Copy)]
pub enum PauseMenuIntent {
    MoveUp,
    MoveDown,
    Select,
    Back,
}

pub fn pause_menu_intent_for_key(key: KeyCode) -> Option<PauseMenuIntent> {
    match key {
        KeyCode::Up => Some(PauseMenuIntent::MoveUp),
        KeyCode::Down => Some(PauseMenuIntent::MoveDown),
        KeyCode::Ok => Some(PauseMenuIntent::Select),
        KeyCode::Back | KeyCode::Key0 => Some(PauseMenuIntent::Back),
        _ => None,
    }
}

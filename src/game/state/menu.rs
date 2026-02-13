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
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

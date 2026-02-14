#[derive(Debug)]
pub struct MenuState {
    pub selected: usize,
    pub has_save: bool,
}

impl MenuState {
    pub fn new(has_save: bool) -> Self {
        Self {
            selected: 0,
            has_save,
        }
    }

    pub fn menu_count(&self) -> usize {
        if self.has_save {
            3
        } else {
            2
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

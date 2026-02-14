#[derive(Debug, Default)]
pub struct MenuState {
    pub selected: usize,
}

impl MenuState {
    pub fn menu_count(&self) -> usize {
        if crate::game::has_save_data() { 3 } else { 2 }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

use wipi::event::KeyCode;

#[derive(Default)]
pub struct InventoryState {
    pub selected: usize,
    pub scroll: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum InventoryIntent {
    MoveUp,
    MoveDown,
    UseSelected,
    Back,
}

impl InventoryState {
    pub fn intent_for_key(key: KeyCode) -> Option<InventoryIntent> {
        match key {
            KeyCode::Up => Some(InventoryIntent::MoveUp),
            KeyCode::Down => Some(InventoryIntent::MoveDown),
            KeyCode::Ok => Some(InventoryIntent::UseSelected),
            KeyCode::Back => Some(InventoryIntent::Back),
            _ => None,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll {
                self.scroll = self.selected;
            }
        }
    }

    pub fn move_down(&mut self, item_count: usize, visible_items: usize) {
        if item_count > 0 && self.selected < item_count - 1 {
            self.selected += 1;
            if self.selected >= self.scroll + visible_items {
                self.scroll = self.selected - visible_items + 1;
            }
        }
    }
}

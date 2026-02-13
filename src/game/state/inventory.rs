#[derive(Debug, Clone, Default)]
pub struct InventoryState {
    pub selected: usize,
    pub scroll: usize,
}

impl InventoryState {
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

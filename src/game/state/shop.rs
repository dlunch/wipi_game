use alloc::vec::Vec;

use crate::data::{Item, Shop};

#[derive(Debug)]
pub struct ShopState {
    pub shop: Shop,
    pub items: Vec<Item>,
    pub selected: usize,
    pub scroll: usize,
    pub mode: ShopMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
    Select,
}

impl ShopState {
    pub fn new(shop: Shop, items: Vec<Item>) -> Self {
        Self {
            shop,
            items,
            selected: 0,
            scroll: 0,
            mode: ShopMode::Select,
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

    pub fn move_down(&mut self, max: usize, visible: usize) {
        if self.selected + 1 < max {
            self.selected += 1;
            if self.selected >= self.scroll + visible {
                self.scroll = self.selected - visible + 1;
            }
        }
    }

    pub fn reset_selection(&mut self) {
        self.selected = 0;
        self.scroll = 0;
    }
}

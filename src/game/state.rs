use crate::data::{Dialog, DialogLine, Item, Shop};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub enum GameState {
    Menu(MenuState),
    Explore,
    Inventory,
    Stats,
    Dialog(DialogState),
    Shop(ShopState),
    QuestLog,
    GameOver,
}

#[derive(Debug, Clone)]
pub struct DialogState {
    pub npc_name: String,
    pub lines: Vec<DialogLine>,
    pub current_line: usize,
}

impl DialogState {
    pub fn new(npc_name: String, dialog: &Dialog) -> Self {
        Self {
            npc_name,
            lines: dialog.lines.clone(),
            current_line: 0,
        }
    }

    pub fn current_text(&self) -> Option<&str> {
        self.lines.get(self.current_line).map(|l| l.text.as_str())
    }

    pub fn advance(&mut self) -> bool {
        if self.current_line + 1 < self.lines.len() {
            self.current_line += 1;
            true
        } else {
            false
        }
    }

    pub fn current_action(&self) -> Option<&crate::data::DialogAction> {
        self.lines
            .get(self.current_line)
            .and_then(|l| l.action.as_ref())
    }
}

#[derive(Debug, Clone)]
pub struct ShopState {
    pub shop: Shop,
    pub items: Vec<Item>,
    pub selected: usize,
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
            mode: ShopMode::Select,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self, max: usize) {
        if self.selected + 1 < max {
            self.selected += 1;
        }
    }
}

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

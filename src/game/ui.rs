use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::data::{Dialog, Item, Shop};

pub const INVENTORY_VISIBLE_ITEMS: usize = 8;
pub const SHOP_VISIBLE_ITEMS: usize = 8;

#[derive(Debug, Default)]
pub struct UiState {
    pub menu: MenuUiState,
    pub pause_menu: PauseMenuUiState,
    pub inventory: InventoryUiState,
    pub shop: ShopUiState,
    pub dialog: DialogUiState,
}

#[derive(Debug)]
pub struct MenuUiState {
    pub state: MenuState,
    pub selected: usize,
}

impl Default for MenuUiState {
    fn default() -> Self {
        Self {
            state: MenuState::new(false),
            selected: 0,
        }
    }
}

impl MenuUiState {
    pub fn set_menu(&mut self, state: MenuState) {
        self.state = state;
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug)]
pub struct PauseMenuUiState {
    pub state: PauseMenuState,
    pub selected: usize,
}

impl Default for PauseMenuUiState {
    fn default() -> Self {
        Self {
            state: PauseMenuState::new(),
            selected: 0,
        }
    }
}

impl PauseMenuUiState {
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug, Default)]
pub struct InventoryUiState {
    pub selected: usize,
}

impl InventoryUiState {
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug)]
pub struct ShopUiState {
    pub state: Option<ShopState>,
    pub mode: ShopMode,
    pub selected: usize,
}

impl Default for ShopUiState {
    fn default() -> Self {
        Self {
            state: None,
            mode: ShopMode::Select,
            selected: 0,
        }
    }
}

impl ShopUiState {
    pub fn open(&mut self, state: ShopState) {
        self.state = Some(state);
        self.mode = ShopMode::Select;
        self.selected = 0;
    }

    pub fn set_mode(&mut self, mode: ShopMode) {
        self.mode = mode;
        self.selected = 0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug, Default)]
pub struct DialogUiState {
    pub state: Option<DialogState>,
}

impl DialogUiState {
    pub fn open(&mut self, state: DialogState) {
        self.state = Some(state);
    }

    pub fn close(&mut self) {
        self.state = None;
    }

    pub fn set(&mut self, state: Option<DialogState>) {
        self.state = state;
    }
}

#[derive(Debug)]
pub struct MenuState {
    pub title: &'static str,
    pub items: Vec<(&'static str, MenuAction)>,
}

impl MenuState {
    pub fn new(has_save: bool) -> Self {
        let items = if has_save {
            vec![
                ("NEW GAME", MenuAction::NewGame),
                ("CONTINUE", MenuAction::Continue),
                ("EXIT", MenuAction::Exit),
            ]
        } else {
            vec![
                ("NEW GAME", MenuAction::NewGame),
                ("EXIT", MenuAction::Exit),
            ]
        };

        Self {
            title: "LOST KINGDOM",
            items,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

#[derive(Debug)]
pub struct PauseMenuState {
    pub items: Vec<&'static str>,
}

impl PauseMenuState {
    pub fn new() -> Self {
        Self {
            items: vec!["Inventory", "Stats", "Quests", "Save"],
        }
    }
}

#[derive(Debug)]
pub struct DialogState {
    pub npc_name: String,
    pub dialog_id: String,
    pub current_line: usize,
}

impl DialogState {
    pub fn new(npc_name: String, dialog: &Dialog) -> Self {
        Self {
            npc_name,
            dialog_id: dialog.id.clone(),
            current_line: 0,
        }
    }
}

#[derive(Debug)]
pub struct ShopState {
    pub shop: Shop,
    pub items: Vec<Item>,
}

impl ShopState {
    pub fn new(shop: Shop, items: Vec<Item>) -> Self {
        Self { shop, items }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
    Select,
}

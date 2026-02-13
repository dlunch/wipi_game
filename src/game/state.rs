mod player;

pub use player::Player;

use alloc::string::String;
use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::data::{Dialog, DialogLine, Item, Map, Shop, Tile};

#[derive(Debug, Clone)]
pub enum GameState {
    Loading(usize),
    Menu(MenuState),
    Explore,
    Inventory,
    Stats,
    Dialog(DialogState),
    Shop(ShopState),
    QuestLog,
    PauseMenu(usize),
    GameOver,
    Error(String),
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

#[derive(Debug, Clone, Default)]
pub struct MenuState {
    pub selected: usize,
    pub has_save: bool,
}

impl MenuState {
    pub fn menu_count(&self) -> usize {
        if self.has_save {
            3
        } else {
            2
        }
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

#[derive(Debug, Clone)]
pub enum TileEvent {
    Treasure,
    MapExit(String),
    DungeonEntrance(String),
}

pub fn check_tile_event(map: &Map, player: &Player) -> Option<TileEvent> {
    let tile = map.get_tile(player.x, player.y);

    match tile {
        Tile::Treasure => Some(TileEvent::Treasure),
        Tile::Exit => {
            for (ex, ey, target) in &map.exits {
                if *ex == player.x && *ey == player.y {
                    return Some(TileEvent::MapExit(target.clone()));
                }
            }
            None
        }
        Tile::Dungeon => {
            for (dx, dy, target) in &map.dungeons {
                if *dx == player.x && *dy == player.y {
                    return Some(TileEvent::DungeonEntrance(target.clone()));
                }
            }
            None
        }
        _ => None,
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
pub enum DialogIntent {
    Confirm,
    Back,
}

#[derive(Debug, Clone, Copy)]
pub enum PauseMenuIntent {
    MoveUp,
    MoveDown,
    Select,
    Back,
}

#[derive(Debug, Clone, Copy)]
pub enum ShopIntent {
    MoveUp,
    MoveDown,
    Confirm,
    Back,
}

impl MenuState {
    pub fn intent_for_key(key: KeyCode) -> Option<MenuIntent> {
        match key {
            KeyCode::Up => Some(MenuIntent::MoveUp),
            KeyCode::Down => Some(MenuIntent::MoveDown),
            KeyCode::Ok => Some(MenuIntent::Select),
            _ => None,
        }
    }
}

impl DialogState {
    pub fn intent_for_key(key: KeyCode) -> Option<DialogIntent> {
        match key {
            KeyCode::Ok => Some(DialogIntent::Confirm),
            KeyCode::Back => Some(DialogIntent::Back),
            _ => None,
        }
    }
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

impl ShopState {
    pub fn intent_for_key(key: KeyCode) -> Option<ShopIntent> {
        match key {
            KeyCode::Up => Some(ShopIntent::MoveUp),
            KeyCode::Down => Some(ShopIntent::MoveDown),
            KeyCode::Ok => Some(ShopIntent::Confirm),
            KeyCode::Back => Some(ShopIntent::Back),
            _ => None,
        }
    }
}

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

mod dialog;
mod inventory;
mod menu;
mod player;
mod shop;
mod tile_event;

pub use dialog::{DialogIntent, DialogState};
pub use inventory::{InventoryIntent, InventoryState};
pub use menu::{MenuAction, MenuIntent, MenuState, PauseMenuIntent, pause_menu_intent_for_key};
pub use player::Player;
pub use shop::{ShopIntent, ShopMode, ShopState};
pub use tile_event::{TileEvent, check_tile_event};

use alloc::string::String;

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

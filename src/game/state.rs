mod dialog;
mod inventory;
mod menu;
mod player;
mod shop;
mod tile_event;

pub use dialog::DialogState;
pub use inventory::InventoryState;
pub use menu::{MenuAction, MenuState};
pub use player::PlayerState;
pub use shop::{ShopMode, ShopState};
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

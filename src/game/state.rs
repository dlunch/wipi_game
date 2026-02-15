mod player;

pub use player::PlayerState;

use alloc::string::String;

#[derive(Debug)]
pub enum GameState {
    Loading(usize),
    Menu,
    Explore,
    Inventory,
    Stats,
    Dialog,
    Shop,
    QuestLog,
    PauseMenu,
    GameOver,
    Error(String),
}

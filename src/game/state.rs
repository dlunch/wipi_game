mod combat;
mod movement;
mod player;

pub use combat::{CombatAction, CombatEvent, CombatState, FieldEnemy, PlayerEffect};
pub use movement::MovementState;
pub use player::{PlayerAction, PlayerEvent, PlayerState, TileApplyEvent, TileEvent};

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

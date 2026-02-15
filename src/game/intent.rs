use wipi::event::KeyCode;

use crate::game::{
    DialogIntent, ExploreIntent, InventoryIntent, MenuIntent, PauseMenuIntent, ShopIntent,
};

#[derive(Debug, Clone, Copy)]
pub enum AppAction {
    Tick,
    KeyDown(KeyCode),
    KeyUp(KeyCode),
}

#[derive(Debug, Clone, Copy)]
pub enum AppIntent {
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    Menu(MenuIntent),
    Explore(ExploreIntent),
    Inventory(InventoryIntent),
    Dialog(DialogIntent),
    Shop(ShopIntent),
    PauseMenu(PauseMenuIntent),
    ReturnToExplore,
    ReturnToMenuFromGameOver,
    ReleaseMovementKey(KeyCode),
    Exit(i32),
}

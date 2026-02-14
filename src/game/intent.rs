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
pub enum AppEffect {
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    ApplyMenuIntent(MenuIntent),
    ApplyExploreIntent(ExploreIntent),
    ApplyInventoryIntent(InventoryIntent),
    ApplyDialogIntent(DialogIntent),
    ApplyShopIntent(ShopIntent),
    ApplyPauseMenuIntent(PauseMenuIntent),
    ReturnToExplore,
    ReturnToMenuFromGameOver,
    ReleaseMovementKey(KeyCode),
    Exit(i32),
}

use crate::game::{
    DialogIntent, ExploreIntent, InventoryIntent, MenuIntent, PauseMenuIntent, ShopIntent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKey {
    Ok,
    Back,
    Up,
    Down,
    Left,
    Right,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
}

#[derive(Debug, Clone, Copy)]
pub enum GameInput {
    Tick,
    KeyDown(InputKey),
    KeyUp(InputKey),
}

#[derive(Debug, Clone, Copy)]
pub enum GameIntent {
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
    ReleaseMovementDirection(crate::data::Direction),
    Exit(i32),
}

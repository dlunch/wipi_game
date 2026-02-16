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

impl InputKey {
    pub fn direction(self) -> Option<crate::data::Direction> {
        match self {
            InputKey::Up => Some(crate::data::Direction::Up),
            InputKey::Down => Some(crate::data::Direction::Down),
            InputKey::Left => Some(crate::data::Direction::Left),
            InputKey::Right => Some(crate::data::Direction::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameInput {
    Tick,
    KeyDown(InputKey),
    KeyUp(InputKey),
}

#[derive(Debug, Clone, Copy)]
pub enum GameIntent {
    System(SystemIntent),
    Scene(SceneIntent),
}

#[derive(Debug, Clone, Copy)]
pub enum SystemIntent {
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    ReturnToExplore,
    ReturnToMenuFromGameOver,
    ReleaseMovementDirection(crate::data::Direction),
    Exit(i32),
}

#[derive(Debug, Clone, Copy)]
pub enum SceneIntent {
    Menu(MenuIntent),
    Explore(ExploreIntent),
    Inventory(InventoryIntent),
    Dialog(DialogIntent),
    Shop(ShopIntent),
    PauseMenu(PauseMenuIntent),
}

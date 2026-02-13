use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::game::{DialogIntent, InventoryIntent, MenuIntent, PauseMenuIntent, ShopIntent};

pub enum AppAction {
    Tick,
    KeyDown(KeyCode),
    KeyUp(KeyCode),
}

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

#[derive(Clone, Copy)]
pub enum ExploreIntent {
    MoveDirection(KeyCode),
    TryNpcInteract,
    Attack,
    Skill1,
    Skill2,
    Skill3,
    Pause,
    BackToMenu,
}

pub fn explore_intents_for_key(key: KeyCode) -> Vec<ExploreIntent> {
    let mut intents = Vec::new();
    match key {
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            intents.push(ExploreIntent::MoveDirection(key));
        }
        KeyCode::Ok => {
            intents.push(ExploreIntent::TryNpcInteract);
            intents.push(ExploreIntent::Attack);
        }
        KeyCode::Key1 => intents.push(ExploreIntent::Skill1),
        KeyCode::Key2 => intents.push(ExploreIntent::Skill2),
        KeyCode::Key3 => intents.push(ExploreIntent::Skill3),
        KeyCode::Key0 => intents.push(ExploreIntent::Pause),
        KeyCode::Back => intents.push(ExploreIntent::BackToMenu),
        _ => {}
    }

    intents
}

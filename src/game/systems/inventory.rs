use wipi::event::KeyCode;
use wipi::framebuffer::Framebuffer;

use crate::game::{GameState, InventoryState, PlayerIntent, PlayerState};

#[derive(Debug, Clone, Copy)]
pub enum InventoryIntent {
    MoveUp,
    MoveDown,
    UseSelected,
    Back,
}

impl InventoryIntent {
    pub fn intent_for_key(key: KeyCode) -> Option<InventoryIntent> {
        match key {
            KeyCode::Up => Some(InventoryIntent::MoveUp),
            KeyCode::Down => Some(InventoryIntent::MoveDown),
            KeyCode::Ok => Some(InventoryIntent::UseSelected),
            KeyCode::Back => Some(InventoryIntent::Back),
            _ => None,
        }
    }
}

pub fn reduce(
    state: &mut GameState,
    player: &mut PlayerState,
    inventory_state: &mut InventoryState,
    intent: InventoryIntent,
) {
    match intent {
        InventoryIntent::MoveUp => inventory_state.move_up(),
        InventoryIntent::MoveDown => {
            let fb = Framebuffer::screen_framebuffer();
            let visible = ((fb.height() as i32 - 50) / 14).max(1) as usize;
            inventory_state.move_down(player.inventory.len(), visible);
        }
        InventoryIntent::UseSelected => {
            let _ = super::player::reduce(
                player,
                PlayerIntent::UseItem {
                    index: inventory_state.selected,
                },
            );
        }
        InventoryIntent::Back => *state = GameState::Explore,
    }
}

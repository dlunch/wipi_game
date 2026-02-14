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
        InventoryIntent::MoveUp => {
            if inventory_state.selected > 0 {
                inventory_state.selected -= 1;
                if inventory_state.selected < inventory_state.scroll {
                    inventory_state.scroll = inventory_state.selected;
                }
            }
        }
        InventoryIntent::MoveDown => {
            let fb = Framebuffer::screen_framebuffer();
            let visible = ((fb.height() as i32 - 50) / 14).max(1) as usize;
            if !player.inventory.is_empty() && inventory_state.selected < player.inventory.len() - 1
            {
                inventory_state.selected += 1;
                if inventory_state.selected >= inventory_state.scroll + visible {
                    inventory_state.scroll = inventory_state.selected - visible + 1;
                }
            }
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

use wipi::event::KeyCode;

use crate::game::{GameState, InventoryUiState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryIntent {
    MoveUp,
    MoveDown,
    UseSelected,
    Back,
}

pub enum InventoryEvent {
    None,
    SetSelected(usize),
    UseSelected(usize),
    CloseToExplore,
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
    state: &GameState,
    inventory_state: &InventoryUiState,
    inventory_len: usize,
    intent: InventoryIntent,
) -> InventoryEvent {
    if !matches!(*state, GameState::Inventory) {
        return InventoryEvent::None;
    }

    match intent {
        InventoryIntent::MoveUp => {
            if inventory_state.selected > 0 {
                return InventoryEvent::SetSelected(inventory_state.selected - 1);
            }
        }
        InventoryIntent::MoveDown => {
            if inventory_len > 0 && inventory_state.selected < inventory_len - 1 {
                return InventoryEvent::SetSelected(inventory_state.selected + 1);
            }
        }
        InventoryIntent::UseSelected => {
            return InventoryEvent::UseSelected(inventory_state.selected)
        }
        InventoryIntent::Back => return InventoryEvent::CloseToExplore,
    }

    InventoryEvent::None
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;
    use crate::data::{Item, ItemKind};

    fn make_potion() -> Item {
        Item {
            id: String::from("potion"),
            name: String::from("Potion"),
            kind: ItemKind::Consumable,
            param1: 30,
            param2: 0,
            param3: 0,
            price: 50,
        }
    }

    #[test]
    fn test_intent_for_key_up() {
        let intent = InventoryIntent::intent_for_key(KeyCode::Up);
        assert_eq!(intent, Some(InventoryIntent::MoveUp));
    }

    #[test]
    fn test_intent_for_key_down() {
        let intent = InventoryIntent::intent_for_key(KeyCode::Down);
        assert_eq!(intent, Some(InventoryIntent::MoveDown));
    }

    #[test]
    fn test_intent_for_key_ok() {
        let intent = InventoryIntent::intent_for_key(KeyCode::Ok);
        assert_eq!(intent, Some(InventoryIntent::UseSelected));
    }

    #[test]
    fn test_intent_for_key_back() {
        let intent = InventoryIntent::intent_for_key(KeyCode::Back);
        assert_eq!(intent, Some(InventoryIntent::Back));
    }

    #[test]
    fn test_intent_for_key_other() {
        let intent = InventoryIntent::intent_for_key(KeyCode::Key0);
        assert_eq!(intent, None);
    }

    #[test]
    fn test_move_up_decrements_selected() {
        let state = GameState::Inventory;
        let mut inventory_state = InventoryUiState::default();
        inventory_state.selected = 2;

        let event = reduce(&state, &inventory_state, 3, InventoryIntent::MoveUp);
        assert!(matches!(event, InventoryEvent::SetSelected(1)));
    }

    #[test]
    fn test_move_up_clamps_at_zero() {
        let state = GameState::Inventory;
        let mut inventory_state = InventoryUiState::default();
        inventory_state.selected = 0;

        let event = reduce(&state, &inventory_state, 3, InventoryIntent::MoveUp);
        assert!(matches!(event, InventoryEvent::None));
    }

    #[test]
    fn test_move_up_decrements_only_selected() {
        let state = GameState::Inventory;
        let mut inventory_state = InventoryUiState::default();
        inventory_state.selected = 3;

        let event = reduce(&state, &inventory_state, 4, InventoryIntent::MoveUp);
        assert!(matches!(event, InventoryEvent::SetSelected(2)));
    }

    #[test]
    fn test_use_selected_heals_player() {
        let state = GameState::Inventory;
        let mut inventory_state = InventoryUiState::default();
        inventory_state.selected = 0;

        let event = reduce(&state, &inventory_state, 1, InventoryIntent::UseSelected);
        assert!(matches!(event, InventoryEvent::UseSelected(0)));
    }

    #[test]
    fn test_back_changes_state_to_explore() {
        let state = GameState::Inventory;
        let inventory_state = InventoryUiState::default();
        let event = reduce(&state, &inventory_state, 0, InventoryIntent::Back);
        assert!(matches!(event, InventoryEvent::CloseToExplore));
    }
}

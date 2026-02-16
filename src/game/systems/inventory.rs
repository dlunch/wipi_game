use crate::game::selection::{step_down, step_up};
use crate::game::systems::runtime::DomainEventResolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryIntent {
    MoveUp,
    MoveDown,
    UseSelected,
    Back,
}

#[derive(Clone)]
pub enum InventoryEvent {
    None,
    SetSelected(usize),
    UseSelected(usize),
    CloseToExplore,
}

pub fn resolve(selected: usize, inventory_len: usize, intent: InventoryIntent) -> InventoryEvent {
    match intent {
        InventoryIntent::MoveUp => {
            let next = step_up(selected);
            if next != selected {
                return InventoryEvent::SetSelected(next);
            }
        }
        InventoryIntent::MoveDown => {
            let next = step_down(selected, inventory_len);
            if next != selected {
                return InventoryEvent::SetSelected(next);
            }
        }
        InventoryIntent::UseSelected => {
            return InventoryEvent::UseSelected(selected);
        }
        InventoryIntent::Back => return InventoryEvent::CloseToExplore,
    }

    InventoryEvent::None
}

pub fn resolve_many(
    selected: usize,
    inventory_len: usize,
    intent: InventoryIntent,
) -> alloc::vec::Vec<InventoryEvent> {
    match resolve(selected, inventory_len, intent) {
        InventoryEvent::None => alloc::vec::Vec::new(),
        event => alloc::vec![event],
    }
}

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec::Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_up_decrements_selected() {
        let event = resolve(2, 3, InventoryIntent::MoveUp);
        assert!(matches!(event, InventoryEvent::SetSelected(1)));
    }

    #[test]
    fn test_move_up_clamps_at_zero() {
        let event = resolve(0, 3, InventoryIntent::MoveUp);
        assert!(matches!(event, InventoryEvent::None));
    }

    #[test]
    fn test_move_up_decrements_only_selected() {
        let event = resolve(3, 4, InventoryIntent::MoveUp);
        assert!(matches!(event, InventoryEvent::SetSelected(2)));
    }

    #[test]
    fn test_use_selected_heals_player() {
        let event = resolve(0, 1, InventoryIntent::UseSelected);
        assert!(matches!(event, InventoryEvent::UseSelected(0)));
    }

    #[test]
    fn test_back_changes_state_to_explore() {
        let event = resolve(0, 0, InventoryIntent::Back);
        assert!(matches!(event, InventoryEvent::CloseToExplore));
    }
}

use crate::game::InputKey;

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
    pub fn intent_for_key(key: InputKey) -> Option<InventoryIntent> {
        match key {
            InputKey::Up => Some(InventoryIntent::MoveUp),
            InputKey::Down => Some(InventoryIntent::MoveDown),
            InputKey::Ok => Some(InventoryIntent::UseSelected),
            InputKey::Back => Some(InventoryIntent::Back),
            _ => None,
        }
    }
}

pub fn resolve(selected: usize, inventory_len: usize, intent: InventoryIntent) -> InventoryEvent {
    match intent {
        InventoryIntent::MoveUp => {
            if selected > 0 {
                return InventoryEvent::SetSelected(selected - 1);
            }
        }
        InventoryIntent::MoveDown => {
            if inventory_len > 0 && selected < inventory_len - 1 {
                return InventoryEvent::SetSelected(selected + 1);
            }
        }
        InventoryIntent::UseSelected => {
            return InventoryEvent::UseSelected(selected);
        }
        InventoryIntent::Back => return InventoryEvent::CloseToExplore,
    }

    InventoryEvent::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_for_key_up() {
        let intent = InventoryIntent::intent_for_key(InputKey::Up);
        assert_eq!(intent, Some(InventoryIntent::MoveUp));
    }

    #[test]
    fn test_intent_for_key_down() {
        let intent = InventoryIntent::intent_for_key(InputKey::Down);
        assert_eq!(intent, Some(InventoryIntent::MoveDown));
    }

    #[test]
    fn test_intent_for_key_ok() {
        let intent = InventoryIntent::intent_for_key(InputKey::Ok);
        assert_eq!(intent, Some(InventoryIntent::UseSelected));
    }

    #[test]
    fn test_intent_for_key_back() {
        let intent = InventoryIntent::intent_for_key(InputKey::Back);
        assert_eq!(intent, Some(InventoryIntent::Back));
    }

    #[test]
    fn test_intent_for_key_other() {
        let intent = InventoryIntent::intent_for_key(InputKey::Key0);
        assert_eq!(intent, None);
    }

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

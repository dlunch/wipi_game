use wipi::event::KeyCode;

use crate::game::{GameState, InventoryState, PlayerIntent, PlayerState};

const VISIBLE_ITEMS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            if !player.inventory.is_empty() && inventory_state.selected < player.inventory.len() - 1
            {
                inventory_state.selected += 1;
                if inventory_state.selected >= inventory_state.scroll + VISIBLE_ITEMS {
                    inventory_state.scroll = inventory_state.selected - VISIBLE_ITEMS + 1;
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
        let mut state = GameState::Inventory;
        let mut player = PlayerState::new(String::from("Hero"), "v");
        let mut inventory_state = InventoryState::default();
        inventory_state.selected = 2;

        reduce(
            &mut state,
            &mut player,
            &mut inventory_state,
            InventoryIntent::MoveUp,
        );

        assert_eq!(inventory_state.selected, 1);
    }

    #[test]
    fn test_move_up_clamps_at_zero() {
        let mut state = GameState::Inventory;
        let mut player = PlayerState::new(String::from("Hero"), "v");
        let mut inventory_state = InventoryState::default();
        inventory_state.selected = 0;

        reduce(
            &mut state,
            &mut player,
            &mut inventory_state,
            InventoryIntent::MoveUp,
        );

        assert_eq!(inventory_state.selected, 0);
    }

    #[test]
    fn test_move_up_adjusts_scroll() {
        let mut state = GameState::Inventory;
        let mut player = PlayerState::new(String::from("Hero"), "v");
        let mut inventory_state = InventoryState::default();
        inventory_state.selected = 3;
        inventory_state.scroll = 3;

        reduce(
            &mut state,
            &mut player,
            &mut inventory_state,
            InventoryIntent::MoveUp,
        );

        assert_eq!(inventory_state.selected, 2);
        assert_eq!(inventory_state.scroll, 2);
    }

    #[test]
    fn test_use_selected_heals_player() {
        let mut state = GameState::Inventory;
        let mut player = PlayerState::new(String::from("Hero"), "v");
        player.stats.current_hp = 50;
        player.stats.max_hp = 80;
        player.inventory.push(make_potion());
        let mut inventory_state = InventoryState::default();
        inventory_state.selected = 0;

        reduce(
            &mut state,
            &mut player,
            &mut inventory_state,
            InventoryIntent::UseSelected,
        );

        assert_eq!(player.stats.current_hp, 80);
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn test_back_changes_state_to_explore() {
        let mut state = GameState::Inventory;
        let mut player = PlayerState::new(String::from("Hero"), "v");
        let mut inventory_state = InventoryState::default();

        reduce(
            &mut state,
            &mut player,
            &mut inventory_state,
            InventoryIntent::Back,
        );

        assert!(matches!(state, GameState::Explore));
    }
}

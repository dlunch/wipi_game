use crate::game::selection::{step_down, step_up};
use anyhow::{Result, anyhow, ensure};

use crate::engine::GameEngine;
use crate::game::systems::runtime::{DomainEventApplier, DomainEventResolver};
use crate::game::{GameState, RuntimeEvent};

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

struct InventoryInputResolver;
struct InventoryApplier;

static INVENTORY_INPUT_RESOLVER: InventoryInputResolver = InventoryInputResolver;
static INVENTORY_APPLIER: InventoryApplier = InventoryApplier;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&INVENTORY_INPUT_RESOLVER]
}

pub fn appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&INVENTORY_APPLIER]
}

impl DomainEventResolver for InventoryInputResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::InventoryInput(_))
    }

    fn resolve(
        &self,
        engine: &mut GameEngine,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::InventoryInput(intent) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        ensure!(
            matches!(engine.state(), GameState::Inventory),
            "Invalid state: expected Inventory"
        );
        let s = engine
            .session()
            .ok_or_else(|| anyhow!("No active session"))?;

        Ok(resolve_many(
            engine.ui().inventory.selected,
            s.player.inventory.len(),
            *intent,
        )
        .into_iter()
        .map(RuntimeEvent::Inventory)
        .collect())
    }
}

impl DomainEventApplier for InventoryApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Inventory(_))
    }

    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Inventory(event) = event else {
            return Ok(());
        };

        match event {
            InventoryEvent::None => {}
            InventoryEvent::SetSelected(selected) => {
                engine.ui_mut().inventory.set_selected(*selected)
            }
            InventoryEvent::UseSelected(index) => {
                let s = engine
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.use_inventory_item(*index);
            }
            InventoryEvent::CloseToExplore => engine.transition_to(GameState::Explore),
        }

        Ok(())
    }
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

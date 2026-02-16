use anyhow::{anyhow, ensure};

use crate::game::selection::{step_down, step_up};
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameEvent, GameState, InputKey};

#[derive(Clone)]
pub enum InventoryEvent {
    None,
    SetSelected(usize),
    UseSelected(usize),
    CloseToExplore,
}

struct InventoryInputResolver;

static INVENTORY_INPUT_RESOLVER: InventoryInputResolver = InventoryInputResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&INVENTORY_INPUT_RESOLVER]
}

impl DomainEventResolver for InventoryInputResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::InventoryInput(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
    ) -> anyhow::Result<alloc::vec::Vec<GameEvent>> {
        let GameEvent::InventoryInput(key) = event else {
            return Err(anyhow!("Invalid event: expected InventoryInput"));
        };
        ensure!(
            matches!(ctx.state, GameState::Inventory),
            "Invalid state: expected Inventory"
        );
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        let selected = ctx.ui.inventory.selected;
        let event = match key {
            InputKey::Up => {
                let next = step_up(selected);
                if next != selected {
                    InventoryEvent::SetSelected(next)
                } else {
                    InventoryEvent::None
                }
            }
            InputKey::Down => {
                let next = step_down(selected, s.player.inventory.len());
                if next != selected {
                    InventoryEvent::SetSelected(next)
                } else {
                    InventoryEvent::None
                }
            }
            InputKey::Ok => InventoryEvent::UseSelected(selected),
            InputKey::Back => InventoryEvent::CloseToExplore,
            _ => InventoryEvent::None,
        };

        match event {
            InventoryEvent::None => Ok(alloc::vec::Vec::new()),
            event => Ok(alloc::vec![GameEvent::Inventory(event)]),
        }
    }
}

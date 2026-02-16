use alloc::vec::Vec;

use anyhow::Result;

use crate::engine::GameEngine;
use crate::game::RuntimeEvent;

pub trait DomainEventResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool;
    fn resolve(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>>;
}

pub trait DomainEventApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool;
    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()>;
}

pub mod combat;
pub mod dialog;
pub mod explore;
pub mod inventory;
pub mod lifecycle;
pub mod menu;
pub mod movement;
pub mod npc;
pub mod runtime;
pub mod shop;

use alloc::vec;
use alloc::vec::Vec;

pub use dialog::{DialogEvent, DialogTransition};
pub use inventory::InventoryEvent;
pub use lifecycle::{LoadingEvent, apply_lifecycle_event};
pub use menu::{MenuEvent, PauseMenuEvent};
pub use npc::NpcEvent;
pub use runtime::{DomainEventResolver, ResolveContext};
pub use shop::ShopEvent;

pub fn domain_resolvers() -> Vec<&'static dyn DomainEventResolver> {
    let mut handlers: Vec<&'static dyn DomainEventResolver> = vec![];
    handlers.extend(lifecycle::resolvers());
    handlers.extend(movement::resolvers());
    handlers.extend(combat::resolvers());
    handlers.extend(menu::resolvers());
    handlers.extend(explore::resolvers());
    handlers.extend(inventory::resolvers());
    handlers.extend(dialog::resolvers());
    handlers.extend(shop::resolvers());
    handlers.extend(npc::resolvers());
    handlers
}

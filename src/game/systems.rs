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
pub mod transition;

use alloc::vec;
use alloc::vec::Vec;

pub use dialog::{DialogEvent, DialogIntent, DialogTransition};
pub use explore::ExploreIntent;
pub use inventory::{InventoryEvent, InventoryIntent};
pub use lifecycle::LoadingEvent;
pub use menu::{MenuEvent, MenuIntent, PauseMenuEvent, PauseMenuIntent};
pub use npc::NpcEvent;
pub use runtime::{DomainEventApplier, DomainEventResolver};
pub use shop::{ShopEvent, ShopIntent};

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
    handlers.extend(transition::resolvers());
    handlers
}

pub fn domain_appliers() -> Vec<&'static dyn DomainEventApplier> {
    let mut handlers: Vec<&'static dyn DomainEventApplier> = vec![];
    handlers.extend(lifecycle::appliers());
    handlers.extend(movement::appliers());
    handlers.extend(combat::appliers());
    handlers.extend(menu::appliers());
    handlers.extend(explore::appliers());
    handlers.extend(inventory::appliers());
    handlers.extend(dialog::appliers());
    handlers.extend(shop::appliers());
    handlers.extend(npc::appliers());
    handlers.extend(transition::appliers());
    handlers
}

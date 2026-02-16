pub mod combat;
pub mod explore;
pub mod lifecycle;
pub mod movement;
pub mod npc;
pub mod runtime;
pub mod shop;

use alloc::vec;
use alloc::vec::Vec;

pub use lifecycle::{LifecycleEvent, LoadingEvent};
pub use npc::NpcEvent;
pub use runtime::{DomainEventResolver, ResolveContext};

pub fn domain_resolvers() -> Vec<&'static dyn DomainEventResolver> {
    let mut handlers: Vec<&'static dyn DomainEventResolver> = vec![];
    handlers.extend(lifecycle::resolvers());
    handlers.extend(movement::resolvers());
    handlers.extend(combat::resolvers());
    handlers.extend(explore::resolvers());
    handlers.extend(shop::resolvers());
    handlers.extend(npc::resolvers());
    handlers
}

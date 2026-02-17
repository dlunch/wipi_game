pub mod character;
pub mod combat;
pub mod lifecycle;
pub mod movement;
pub mod npc;
pub mod resolver;
pub mod shop;
pub mod world;

use alloc::vec;
use alloc::vec::Vec;

pub use lifecycle::{LifecycleEvent, LoadingEvent};
pub use npc::NpcEvent;
pub use resolver::DomainEventResolver;

pub fn domain_resolvers() -> Vec<&'static dyn DomainEventResolver> {
    let mut handlers: Vec<&'static dyn DomainEventResolver> = vec![];
    handlers.extend(lifecycle::resolvers());
    handlers.extend(movement::resolvers());
    handlers.extend(combat::resolvers());
    handlers.extend(world::resolvers());
    handlers.extend(character::resolvers());
    handlers.extend(shop::resolvers());
    handlers.extend(npc::resolvers());
    handlers
}

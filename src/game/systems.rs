pub mod character;
pub mod combat;
pub mod explore;
pub mod lifecycle;
pub mod movement;
pub mod npc;
pub mod resolver;
pub mod shop;
pub mod world;

use alloc::vec;
use alloc::vec::Vec;

pub use lifecycle::{LifecycleEvent, LoadingEvent, load_step, resolve_loading};
pub use npc::NpcEvent;
pub use resolver::{DomainEventResolver, ResolveContext};

pub fn domain_resolvers() -> Vec<&'static dyn DomainEventResolver> {
    let mut handlers: Vec<&'static dyn DomainEventResolver> = vec![];
    for resolver in lifecycle::resolvers() {
        handlers.push(resolver);
    }
    for resolver in movement::resolvers() {
        handlers.push(resolver);
    }
    for resolver in combat::resolvers() {
        handlers.push(resolver);
    }
    for resolver in world::resolvers() {
        handlers.push(resolver);
    }
    for resolver in character::resolvers() {
        handlers.push(resolver);
    }
    for resolver in explore::resolvers() {
        handlers.push(resolver);
    }
    for resolver in shop::resolvers() {
        handlers.push(resolver);
    }
    for resolver in npc::resolvers() {
        handlers.push(resolver);
    }
    handlers
}

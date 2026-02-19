pub(crate) mod character;
pub(crate) mod combat;
mod emit;
pub(crate) mod lifecycle;
pub(crate) mod movement;
pub(crate) mod npc;
pub(crate) mod resolver;
pub(crate) mod shop;
pub(crate) mod world;

use alloc::vec;
use alloc::vec::Vec;

pub fn domain_resolvers() -> Vec<&'static dyn resolver::DomainEventResolver> {
    let mut handlers: Vec<&'static dyn resolver::DomainEventResolver> = vec![];
    handlers.extend(lifecycle::resolvers());
    handlers.extend(movement::resolvers());
    handlers.extend(combat::resolvers());
    handlers.extend(world::resolvers());
    handlers.extend(character::resolvers());
    handlers.extend(shop::resolvers());
    handlers.extend(npc::resolvers());
    handlers
}

pub(crate) mod character;
pub(crate) mod combat;
mod emit;
pub(crate) mod lifecycle;
pub(crate) mod movement;
pub(crate) mod npc;
pub(crate) mod resolver;
pub(crate) mod shop;
pub(crate) mod world;

use alloc::vec::Vec;

pub fn domain_resolvers() -> Vec<&'static dyn resolver::DomainEventResolver> {
    let mut resolvers = Vec::new();
    resolvers.extend(lifecycle::resolvers());
    resolvers.extend(movement::resolvers());
    resolvers.extend(combat::resolvers());
    resolvers.extend(world::resolvers());
    resolvers.extend(character::resolvers());
    resolvers.extend(shop::resolvers());
    resolvers.extend(npc::resolvers());
    resolvers
}

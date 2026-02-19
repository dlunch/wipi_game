pub mod character;
pub mod combat;
mod emit;
pub mod lifecycle;
pub mod movement;
pub mod npc;
pub mod resolver;
pub mod world;
mod world_shop;

use alloc::vec::Vec;

pub fn domain_resolvers() -> Vec<&'static dyn resolver::DomainEventResolver> {
    let mut resolvers = Vec::new();
    resolvers.extend(lifecycle::resolvers());
    resolvers.extend(movement::resolvers());
    resolvers.extend(combat::resolvers());
    resolvers.extend(world::resolvers());
    resolvers.extend(character::resolvers());
    resolvers.extend(npc::resolvers());
    resolvers
}

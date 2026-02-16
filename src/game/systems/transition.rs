use crate::game::systems::runtime::DomainEventResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec::Vec::new()
}

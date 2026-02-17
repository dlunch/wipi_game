use alloc::vec;
use alloc::vec::Vec;

use crate::game::systems::resolver::DomainEventResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![]
}

use crate::game::systems::runtime::DomainEventApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    let mut handlers: alloc::vec::Vec<&'static dyn DomainEventApplier> = alloc::vec![];
    handlers.extend(super::state_appliers());
    handlers.extend(super::combat::domain_appliers());
    handlers.extend(super::movement::domain_appliers());
    handlers.extend(super::session::domain_appliers());
    handlers.extend(super::ui::domain_appliers());
    handlers
}

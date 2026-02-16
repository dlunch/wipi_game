use crate::data::Item;
use crate::game::systems::runtime::DomainEventResolver;

#[derive(Clone)]
pub enum ShopEvent {
    BuyItem(Item),
    SellSelected(usize),
    CloseToExplore,
}

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec::Vec::new()
}

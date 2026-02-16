use crate::data::Item;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameEvent, GameState, ShopInputEvent};

#[derive(Clone)]
pub enum ShopEvent {
    BuyItem(Item),
    SellSelected(usize),
    CloseToExplore,
}

struct ShopInputResolver;

static SHOP_INPUT_RESOLVER: ShopInputResolver = ShopInputResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&SHOP_INPUT_RESOLVER]
}

impl DomainEventResolver for ShopInputResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::ShopInput(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
    ) -> anyhow::Result<alloc::vec::Vec<GameEvent>> {
        let GameEvent::ShopInput(input) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        if !matches!(ctx.state, GameState::Shop) {
            return Ok(alloc::vec::Vec::new());
        }
        let Some(s) = ctx.session else {
            return Ok(alloc::vec::Vec::new());
        };

        let event = match input {
            ShopInputEvent::BuySelected(selected) => {
                let shop_items = ctx
                    .ui
                    .shop
                    .state
                    .as_ref()
                    .map(|state| state.items.as_slice())
                    .unwrap_or(&[]);
                if let Some(item) = shop_items.get(*selected).cloned() {
                    if s.player.stats.gold >= item.price {
                        Some(ShopEvent::BuyItem(item))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            ShopInputEvent::SellSelected(selected) => Some(ShopEvent::SellSelected(*selected)),
            ShopInputEvent::Close => Some(ShopEvent::CloseToExplore),
        };

        if let Some(event) = event {
            Ok(alloc::vec![GameEvent::Shop(event)])
        } else {
            Ok(alloc::vec::Vec::new())
        }
    }
}

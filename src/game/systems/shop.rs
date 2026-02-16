use alloc::boxed::Box;

use anyhow::{anyhow, ensure};

use crate::data::Item;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameEvent, GameState, ShopInputEvent, ShopState};

#[derive(Clone)]
pub enum ShopEvent {
    BuyItem(Item),
    SellSelected(usize),
    CloseToExplore,
}

struct ShopInputResolver;
struct OpenShopByIdResolver;

static SHOP_INPUT_RESOLVER: ShopInputResolver = ShopInputResolver;
static OPEN_SHOP_BY_ID_RESOLVER: OpenShopByIdResolver = OpenShopByIdResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&SHOP_INPUT_RESOLVER, &OPEN_SHOP_BY_ID_RESOLVER]
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
            return Err(anyhow!("Invalid event: expected ShopInput"));
        };
        ensure!(
            matches!(ctx.state, GameState::Shop),
            "Invalid state: expected Shop"
        );
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

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

impl DomainEventResolver for OpenShopByIdResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::OpenShopById(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
    ) -> anyhow::Result<alloc::vec::Vec<GameEvent>> {
        let GameEvent::OpenShopById(shop_id) = event else {
            return Err(anyhow!("Invalid event: expected OpenShopById"));
        };

        let Some(shop) = ctx.data().find_shop(shop_id).cloned() else {
            return Err(anyhow!("Shop not found: {shop_id}"));
        };
        let shop_items = ctx.data().get_shop_items(&shop);
        Ok(alloc::vec![GameEvent::OpenShopState(Box::new(
            ShopState::new(shop, shop_items),
        ))])
    }
}

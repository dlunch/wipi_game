use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameEvent, ShopState};
struct OpenShopByIdResolver;

static OPEN_SHOP_BY_ID_RESOLVER: OpenShopByIdResolver = OpenShopByIdResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&OPEN_SHOP_BY_ID_RESOLVER]
}

impl DomainEventResolver for OpenShopByIdResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::OpenShopById(_))
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::OpenShopById(shop_id) = event else {
            return Err(anyhow!("Invalid event: expected OpenShopById"));
        };

        let Some(shop) = ctx.data().find_shop(shop_id).cloned() else {
            return Err(anyhow!("Shop not found: {shop_id}"));
        };
        let shop_items = ctx.data().get_shop_items(&shop);
        Ok(vec![GameEvent::OpenShopState(Box::new(ShopState::new(
            shop, shop_items,
        )))])
    }
}

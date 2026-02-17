use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::systems::resolver::{DomainEventResolver, ResolveContext};
use crate::game::{GameEvent, GameEventKind, ShopState};
struct OpenShopByIdResolver;

static OPEN_SHOP_BY_ID_RESOLVER: OpenShopByIdResolver = OpenShopByIdResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&OPEN_SHOP_BY_ID_RESOLVER]
}

impl DomainEventResolver for OpenShopByIdResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::OpenShopById]
    }

    fn resolve(
        &self,
        ctx: &ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let GameEvent::OpenShopById(shop_id) = event else {
            return Err(anyhow!("Invalid event: expected OpenShopById"));
        };

        let Some(shop) = ctx.data().find_shop(shop_id).cloned() else {
            return Err(anyhow!("Shop not found: {shop_id}"));
        };
        let shop_items = ctx.data().get_shop_items(&shop);
        out.push(GameEvent::OpenShopState(Box::new(ShopState::new(
            shop, shop_items,
        ))));
        Ok(())
    }
}

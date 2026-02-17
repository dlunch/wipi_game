use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{GameData, GameEvent, GameEventKind, ShopState, WorldState};
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
        data: &Rc<GameData>,
        _world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let GameEvent::OpenShopById(shop_id) = event else {
            return Err(anyhow!("Invalid event: expected OpenShopById"));
        };

        let Some(shop) = data.find_shop(shop_id).cloned() else {
            return Err(anyhow!("Shop not found: {shop_id}"));
        };
        let shop_items = data.get_shop_items(&shop);
        out.push(GameEvent::OpenShopState(Box::new(ShopState::new(
            shop, shop_items,
        ))));
        Ok(())
    }
}

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use super::resolver::DomainEventResolver;
use crate::game::game_data::GameData;
use crate::game::game_event::{GameEvent, GameEventKind};
use crate::game::ui::state::ShopState;
use crate::game::world::WorldState;
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

        let shop = data.find_shop(shop_id)?.clone();
        let shop_items = data.get_shop_items(&shop)?;
        out.push(GameEvent::OpenShopState(Box::new(ShopState::new(
            shop, shop_items,
        ))));
        Ok(())
    }
}

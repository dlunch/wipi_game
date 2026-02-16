use crate::data::Item;
use anyhow::{Result, anyhow, ensure};

use crate::engine::GameEngine;
use crate::game::systems::runtime::{DomainEventApplier, DomainEventResolver};
use crate::game::{GameState, RuntimeEvent};

#[derive(Debug, Clone, Copy)]
pub enum ShopIntent {
    BuySelected(usize),
    SellSelected(usize),
    Close,
}

#[derive(Clone)]
pub enum ShopEvent {
    None,
    BuyItem(Item),
    SellSelected(usize),
    CloseToExplore,
}

pub fn resolve(intent: ShopIntent, player_gold: i32, shop_items: &[Item]) -> ShopEvent {
    match intent {
        ShopIntent::BuySelected(selected) => {
            if let Some(item) = shop_items.get(selected).cloned()
                && player_gold >= item.price
            {
                return ShopEvent::BuyItem(item);
            }
            ShopEvent::None
        }
        ShopIntent::SellSelected(selected) => ShopEvent::SellSelected(selected),
        ShopIntent::Close => ShopEvent::CloseToExplore,
    }
}

pub fn resolve_many(
    intent: ShopIntent,
    player_gold: i32,
    shop_items: &[Item],
) -> alloc::vec::Vec<ShopEvent> {
    match resolve(intent, player_gold, shop_items) {
        ShopEvent::None => alloc::vec::Vec::new(),
        event => alloc::vec![event],
    }
}

struct ShopInputResolver;
struct ShopApplier;

static SHOP_INPUT_RESOLVER: ShopInputResolver = ShopInputResolver;
static SHOP_APPLIER: ShopApplier = ShopApplier;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&SHOP_INPUT_RESOLVER]
}

pub fn appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&SHOP_APPLIER]
}

impl DomainEventResolver for ShopInputResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::ShopInput(_))
    }

    fn resolve(
        &self,
        engine: &mut GameEngine,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::ShopInput(intent) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        ensure!(
            matches!(engine.state(), GameState::Shop),
            "Invalid state: expected Shop"
        );
        let s = engine
            .session()
            .ok_or_else(|| anyhow!("No active session"))?;
        let shop_items = engine
            .ui()
            .shop
            .state
            .as_ref()
            .map(|state| state.items.as_slice())
            .unwrap_or(&[]);

        Ok(resolve_many(*intent, s.player.stats.gold, shop_items)
            .into_iter()
            .map(RuntimeEvent::Shop)
            .collect())
    }
}

impl DomainEventApplier for ShopApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Shop(_))
    }

    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Shop(event) = event else {
            return Ok(());
        };
        match event {
            ShopEvent::None => {}
            ShopEvent::BuyItem(item) => {
                let s = engine
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.buy_shop_item(item.clone());
            }
            ShopEvent::SellSelected(index) => {
                let (sold, len_after) = {
                    let s = engine
                        .session_mut()
                        .ok_or_else(|| anyhow!("No active session"))?;
                    let sold = s.sell_inventory_item(*index).is_some();
                    (sold, s.player.inventory.len())
                };
                if sold {
                    let current_selected = engine.ui().shop.selected;
                    if current_selected >= len_after && current_selected > 0 {
                        engine.ui_mut().shop.set_selected(current_selected - 1);
                    }
                }
            }
            ShopEvent::CloseToExplore => engine.transition_to(GameState::Explore),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;

    use super::*;
    use crate::data::{Item, ItemKind, Shop};
    use crate::game::ui::ShopUiState;
    use crate::game::{InputKey, ShopMode, ShopState};

    fn make_item(id: &str, price: i32) -> Item {
        Item {
            id: String::from(id),
            name: String::from(id),
            kind: ItemKind::Consumable,
            param1: 10,
            param2: 0,
            param3: 0,
            price,
        }
    }

    fn make_shop_state() -> ShopState {
        ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![],
            },
            vec![],
        )
    }

    #[test]
    fn ui_select_mode_confirm_switches_modes_without_intent() {
        let shop_state = make_shop_state();
        let mut ui = ShopUiState::default();
        ui.open(shop_state);

        let intent = ui.handle_key(InputKey::Ok, 0);
        assert!(intent.is_none());
        assert!(matches!(ui.mode, ShopMode::Buy));

        ui.mode = ShopMode::Select;
        ui.set_selected(1);
        let intent = ui.handle_key(InputKey::Ok, 0);
        assert!(intent.is_none());
        assert!(matches!(ui.mode, ShopMode::Sell));
    }

    #[test]
    fn resolve_buy_selected_buys_with_enough_gold() {
        let shop_state = ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![String::from("potion")],
            },
            vec![make_item("potion", 10)],
        );

        let event = resolve(ShopIntent::BuySelected(0), 50, &shop_state.items);
        assert!(matches!(event, ShopEvent::BuyItem(_)));
    }

    #[test]
    fn resolve_sell_selected_emits_sell_event() {
        let event = resolve(ShopIntent::SellSelected(0), 0, &[]);
        assert!(matches!(event, ShopEvent::SellSelected(0)));
    }

    #[test]
    fn ui_back_in_buy_or_sell_returns_to_select_without_intent() {
        let shop_state = ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![String::from("potion")],
            },
            vec![make_item("potion", 10)],
        );
        let mut ui = ShopUiState::default();
        ui.open(shop_state);
        ui.mode = ShopMode::Buy;
        ui.set_selected(2);

        let intent = ui.handle_key(InputKey::Back, 0);
        assert!(intent.is_none());
        assert!(matches!(ui.mode, ShopMode::Select));
    }
}

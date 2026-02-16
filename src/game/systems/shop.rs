use crate::data::Item;

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

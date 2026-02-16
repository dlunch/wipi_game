use wipi::event::KeyCode;

use crate::data::Item;
use crate::game::ShopMode;

#[derive(Debug, Clone, Copy)]
pub enum ShopIntent {
    MoveUp,
    MoveDown,
    Confirm,
    Back,
}

pub enum ShopEvent {
    None,
    ErrorNoActiveShop,
    SetMode(ShopMode),
    SetSelected(usize),
    BuyItem(Item),
    SellSelected(usize),
    CloseToExplore,
}

pub enum ShopUiEvent {
    None,
    ErrorNoActiveShop,
    SetMode(ShopMode),
    SetSelected(usize),
    RequestBuy(usize),
    RequestSell(usize),
    CloseToExplore,
}

impl ShopIntent {
    pub fn intent_for_key(key: KeyCode) -> Option<ShopIntent> {
        match key {
            KeyCode::Up => Some(ShopIntent::MoveUp),
            KeyCode::Down => Some(ShopIntent::MoveDown),
            KeyCode::Ok => Some(ShopIntent::Confirm),
            KeyCode::Back => Some(ShopIntent::Back),
            _ => None,
        }
    }
}

pub fn resolve_ui(
    mode: ShopMode,
    selected: usize,
    shop_open: bool,
    player_inventory_len: usize,
    shop_items_len: usize,
    intent: ShopIntent,
) -> ShopUiEvent {
    if !shop_open {
        return ShopUiEvent::ErrorNoActiveShop;
    }

    match mode {
        ShopMode::Select => match intent {
            ShopIntent::MoveUp => {
                if selected > 0 {
                    return ShopUiEvent::SetSelected(selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if selected + 1 < 2 {
                    return ShopUiEvent::SetSelected(selected + 1);
                }
            }
            ShopIntent::Confirm => {
                let mode = if selected == 0 {
                    ShopMode::Buy
                } else {
                    ShopMode::Sell
                };
                return ShopUiEvent::SetMode(mode);
            }
            ShopIntent::Back => return ShopUiEvent::CloseToExplore,
        },
        ShopMode::Buy => match intent {
            ShopIntent::MoveUp => {
                if selected > 0 {
                    return ShopUiEvent::SetSelected(selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if selected + 1 < shop_items_len {
                    return ShopUiEvent::SetSelected(selected + 1);
                }
            }
            ShopIntent::Confirm => {
                return ShopUiEvent::RequestBuy(selected);
            }
            ShopIntent::Back => return ShopUiEvent::SetMode(ShopMode::Select),
        },
        ShopMode::Sell => match intent {
            ShopIntent::MoveUp => {
                if selected > 0 {
                    return ShopUiEvent::SetSelected(selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if selected + 1 < player_inventory_len {
                    return ShopUiEvent::SetSelected(selected + 1);
                }
            }
            ShopIntent::Confirm => return ShopUiEvent::RequestSell(selected),
            ShopIntent::Back => return ShopUiEvent::SetMode(ShopMode::Select),
        },
    }

    ShopUiEvent::None
}

pub fn resolve_buy(selected: usize, player_gold: i32, shop_items: &[Item]) -> Option<Item> {
    if let Some(item) = shop_items.get(selected).cloned()
        && player_gold >= item.price
    {
        return Some(item);
    }
    None
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;

    use super::ShopIntent::{Back, Confirm};
    use super::*;
    use crate::data::{Item, ItemKind, Shop};
    use crate::game::ShopState;

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
    fn select_mode_confirm_enters_buy_or_sell() {
        let shop_state = make_shop_state();

        let event = resolve_ui(
            ShopMode::Select,
            0,
            true,
            0,
            shop_state.items.len(),
            Confirm,
        );
        assert!(matches!(event, ShopUiEvent::SetMode(ShopMode::Buy)));

        let event = resolve_ui(
            ShopMode::Select,
            1,
            true,
            0,
            shop_state.items.len(),
            Confirm,
        );
        assert!(matches!(event, ShopUiEvent::SetMode(ShopMode::Sell)));
    }

    #[test]
    fn buy_mode_confirm_buys_with_enough_gold() {
        let shop_state = ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![String::from("potion")],
            },
            vec![make_item("potion", 10)],
        );

        let ui_event = resolve_ui(ShopMode::Buy, 0, true, 0, shop_state.items.len(), Confirm);
        assert!(matches!(ui_event, ShopUiEvent::RequestBuy(0)));

        let tx_event = resolve_buy(0, 50, &shop_state.items);
        assert!(tx_event.is_some());
    }

    #[test]
    fn sell_mode_confirm_sells_item() {
        let shop_state = ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![],
            },
            vec![],
        );
        let event = resolve_ui(ShopMode::Sell, 0, true, 1, shop_state.items.len(), Confirm);
        assert!(matches!(event, ShopUiEvent::RequestSell(0)));
    }

    #[test]
    fn back_in_buy_or_sell_returns_to_select() {
        let shop_state = ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![String::from("potion")],
            },
            vec![make_item("potion", 10)],
        );

        let event = resolve_ui(ShopMode::Buy, 2, true, 0, shop_state.items.len(), Back);
        assert!(matches!(event, ShopUiEvent::SetMode(ShopMode::Select)));
    }
}

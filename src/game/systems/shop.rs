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

pub fn reduce(
    mode: ShopMode,
    selected: usize,
    shop_open: bool,
    player_gold: i32,
    player_inventory_len: usize,
    shop_items: &[Item],
    intent: ShopIntent,
) -> ShopEvent {
    if !shop_open {
        return ShopEvent::ErrorNoActiveShop;
    }

    match mode {
        ShopMode::Select => match intent {
            ShopIntent::MoveUp => {
                if selected > 0 {
                    return ShopEvent::SetSelected(selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if selected + 1 < 2 {
                    return ShopEvent::SetSelected(selected + 1);
                }
            }
            ShopIntent::Confirm => {
                let mode = if selected == 0 {
                    ShopMode::Buy
                } else {
                    ShopMode::Sell
                };
                return ShopEvent::SetMode(mode);
            }
            ShopIntent::Back => return ShopEvent::CloseToExplore,
        },
        ShopMode::Buy => match intent {
            ShopIntent::MoveUp => {
                if selected > 0 {
                    return ShopEvent::SetSelected(selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if selected + 1 < shop_items.len() {
                    return ShopEvent::SetSelected(selected + 1);
                }
            }
            ShopIntent::Confirm => {
                if let Some(item) = shop_items.get(selected).cloned()
                    && player_gold >= item.price
                {
                    return ShopEvent::BuyItem(item);
                }
            }
            ShopIntent::Back => return ShopEvent::SetMode(ShopMode::Select),
        },
        ShopMode::Sell => match intent {
            ShopIntent::MoveUp => {
                if selected > 0 {
                    return ShopEvent::SetSelected(selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if selected + 1 < player_inventory_len {
                    return ShopEvent::SetSelected(selected + 1);
                }
            }
            ShopIntent::Confirm => return ShopEvent::SellSelected(selected),
            ShopIntent::Back => return ShopEvent::SetMode(ShopMode::Select),
        },
    }

    ShopEvent::None
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

        let event = reduce(ShopMode::Select, 0, true, 0, 0, &shop_state.items, Confirm);
        assert!(matches!(event, ShopEvent::SetMode(ShopMode::Buy)));

        let event = reduce(ShopMode::Select, 1, true, 0, 0, &shop_state.items, Confirm);
        assert!(matches!(event, ShopEvent::SetMode(ShopMode::Sell)));
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

        let event = reduce(ShopMode::Buy, 0, true, 50, 0, &shop_state.items, Confirm);
        assert!(matches!(event, ShopEvent::BuyItem(_)));
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
        let event = reduce(ShopMode::Sell, 0, true, 0, 1, &shop_state.items, Confirm);
        assert!(matches!(event, ShopEvent::SellSelected(0)));
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

        let event = reduce(ShopMode::Buy, 2, true, 0, 0, &shop_state.items, Back);
        assert!(matches!(event, ShopEvent::SetMode(ShopMode::Select)));
    }
}

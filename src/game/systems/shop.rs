use wipi::event::KeyCode;

use crate::data::Item;
use crate::game::{GameState, PlayerState, ShopMode, ShopUiState};

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
    state: &GameState,
    player: &PlayerState,
    ui: &ShopUiState,
    intent: ShopIntent,
) -> ShopEvent {
    let GameState::Shop = *state else {
        return ShopEvent::None;
    };

    let Some(shop_state) = ui.state.as_ref() else {
        return ShopEvent::ErrorNoActiveShop;
    };

    match ui.mode {
        ShopMode::Select => match intent {
            ShopIntent::MoveUp => {
                if ui.selected > 0 {
                    return ShopEvent::SetSelected(ui.selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if ui.selected + 1 < 2 {
                    return ShopEvent::SetSelected(ui.selected + 1);
                }
            }
            ShopIntent::Confirm => {
                let mode = if ui.selected == 0 {
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
                if ui.selected > 0 {
                    return ShopEvent::SetSelected(ui.selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if ui.selected + 1 < shop_state.items.len() {
                    return ShopEvent::SetSelected(ui.selected + 1);
                }
            }
            ShopIntent::Confirm => {
                if let Some(item) = shop_state.items.get(ui.selected).cloned()
                    && player.stats.gold >= item.price
                {
                    return ShopEvent::BuyItem(item);
                }
            }
            ShopIntent::Back => return ShopEvent::SetMode(ShopMode::Select),
        },
        ShopMode::Sell => match intent {
            ShopIntent::MoveUp => {
                if ui.selected > 0 {
                    return ShopEvent::SetSelected(ui.selected - 1);
                }
            }
            ShopIntent::MoveDown => {
                if ui.selected + 1 < player.inventory.len() {
                    return ShopEvent::SetSelected(ui.selected + 1);
                }
            }
            ShopIntent::Confirm => return ShopEvent::SellSelected(ui.selected),
            ShopIntent::Back => return ShopEvent::SetMode(ShopMode::Select),
        },
    }

    ShopEvent::None
}

#[cfg(test)]
fn scroll_for_selection(selected: usize, total: usize) -> usize {
    if total <= crate::game::SHOP_VISIBLE_ITEMS {
        return 0;
    }

    let max_scroll = total.saturating_sub(crate::game::SHOP_VISIBLE_ITEMS);
    selected
        .saturating_sub(crate::game::SHOP_VISIBLE_ITEMS - 1)
        .min(max_scroll)
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

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

    fn make_shop_state(_items: Vec<Item>) -> (GameState, PlayerState) {
        (
            GameState::Shop,
            PlayerState::new(String::from("Hero"), "village"),
        )
    }

    #[test]
    fn select_mode_confirm_enters_buy_or_sell() {
        let (state, player) = make_shop_state(vec![]);
        let mut ui = ShopUiState::default();
        ui.state = Some(ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![],
            },
            vec![],
        ));

        let event = reduce(&state, &player, &ui, Confirm);
        assert!(matches!(event, ShopEvent::SetMode(ShopMode::Buy)));

        ui.mode = ShopMode::Select;
        ui.selected = 1;
        let event = reduce(&state, &player, &ui, Confirm);
        assert!(matches!(event, ShopEvent::SetMode(ShopMode::Sell)));
    }

    #[test]
    fn buy_mode_confirm_buys_with_enough_gold() {
        let (state, mut player) = make_shop_state(vec![make_item("potion", 10)]);
        let mut ui = ShopUiState {
            state: None,
            mode: ShopMode::Buy,
            selected: 0,
        };
        ui.state = Some(ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![String::from("potion")],
            },
            vec![make_item("potion", 10)],
        ));

        player.stats.gold = 50;
        let event = reduce(&state, &player, &ui, Confirm);
        assert!(matches!(event, ShopEvent::BuyItem(_)));
    }

    #[test]
    fn sell_mode_confirm_sells_item() {
        let (state, player) = make_shop_state(vec![]);
        let mut ui = ShopUiState {
            state: None,
            mode: ShopMode::Sell,
            selected: 0,
        };
        ui.state = Some(ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![],
            },
            vec![],
        ));
        let event = reduce(&state, &player, &ui, Confirm);
        assert!(matches!(event, ShopEvent::SellSelected(0)));
    }

    #[test]
    fn back_in_buy_or_sell_returns_to_select() {
        let (state, player) = make_shop_state(vec![make_item("potion", 10)]);
        let mut ui = ShopUiState {
            state: None,
            mode: ShopMode::Buy,
            selected: 2,
        };
        ui.state = Some(ShopState::new(
            Shop {
                id: String::from("shop"),
                name: String::from("Shop"),
                items: vec![String::from("potion")],
            },
            vec![make_item("potion", 10)],
        ));

        let event = reduce(&state, &player, &ui, Back);
        assert!(matches!(event, ShopEvent::SetMode(ShopMode::Select)));
    }

    #[test]
    fn scroll_for_selection_is_derived() {
        assert_eq!(scroll_for_selection(0, 20), 0);
        assert_eq!(scroll_for_selection(7, 20), 0);
        assert_eq!(scroll_for_selection(8, 20), 1);
        assert_eq!(scroll_for_selection(19, 20), 12);
    }
}

use wipi::event::KeyCode;

use crate::game::{self, GameState, PlayerEvent, PlayerIntent, PlayerState, ShopMode, ShopUiState};

#[derive(Debug, Clone, Copy)]
pub enum ShopIntent {
    MoveUp,
    MoveDown,
    Confirm,
    Back,
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
    state: &mut GameState,
    player: &mut PlayerState,
    ui: &mut ShopUiState,
    intent: ShopIntent,
) {
    let GameState::Shop(ref shop_state) = *state else {
        return;
    };

    match ui.mode {
        ShopMode::Select => match intent {
            ShopIntent::MoveUp => {
                if ui.selected > 0 {
                    ui.selected -= 1;
                }
            }
            ShopIntent::MoveDown => {
                if ui.selected + 1 < 2 {
                    ui.selected += 1;
                }
            }
            ShopIntent::Confirm => {
                ui.mode = if ui.selected == 0 {
                    ShopMode::Buy
                } else {
                    ShopMode::Sell
                };
                ui.selected = 0;
            }
            ShopIntent::Back => *state = GameState::Explore,
        },
        ShopMode::Buy => match intent {
            ShopIntent::MoveUp => {
                if ui.selected > 0 {
                    ui.selected -= 1;
                }
            }
            ShopIntent::MoveDown => {
                if ui.selected + 1 < shop_state.items.len() {
                    ui.selected += 1;
                }
            }
            ShopIntent::Confirm => {
                if let Some(item) = shop_state.items.get(ui.selected).cloned()
                    && player.stats.gold >= item.price
                {
                    let _ = game::player::reduce(player, PlayerIntent::AddGold(-item.price));
                    let _ = game::player::reduce(player, PlayerIntent::AddItem(item));
                }
            }
            ShopIntent::Back => {
                ui.mode = ShopMode::Select;
                ui.selected = 0;
            }
        },
        ShopMode::Sell => match intent {
            ShopIntent::MoveUp => {
                if ui.selected > 0 {
                    ui.selected -= 1;
                }
            }
            ShopIntent::MoveDown => {
                if ui.selected + 1 < player.inventory.len() {
                    ui.selected += 1;
                }
            }
            ShopIntent::Confirm => {
                let event = game::player::reduce(player, PlayerIntent::RemoveItemAt(ui.selected));
                if let PlayerEvent::ItemRemoved(Some(item)) = event {
                    let _ = game::player::reduce(player, PlayerIntent::AddGold(item.price / 2));

                    let inv_len = player.inventory.len();
                    if ui.selected >= inv_len && ui.selected > 0 {
                        ui.selected -= 1;
                    }
                }
            }
            ShopIntent::Back => {
                ui.mode = ShopMode::Select;
                ui.selected = 0;
            }
        },
    }
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

    fn make_shop_state(items: Vec<Item>) -> (GameState, PlayerState) {
        let shop = Shop {
            id: String::from("shop"),
            name: String::from("Shop"),
            items: items.iter().map(|item| item.id.clone()).collect(),
        };

        (
            GameState::Shop(ShopState::new(shop, items)),
            PlayerState::new(String::from("Hero"), "village"),
        )
    }

    #[test]
    fn select_mode_confirm_enters_buy_or_sell() {
        let (mut state, mut player) = make_shop_state(vec![]);
        let mut ui = ShopUiState::default();

        reduce(&mut state, &mut player, &mut ui, Confirm);
        assert!(matches!(ui.mode, ShopMode::Buy));

        ui.mode = ShopMode::Select;
        ui.selected = 1;
        reduce(&mut state, &mut player, &mut ui, Confirm);
        assert!(matches!(ui.mode, ShopMode::Sell));
    }

    #[test]
    fn buy_mode_confirm_buys_with_enough_gold() {
        let (mut state, mut player) = make_shop_state(vec![make_item("potion", 10)]);
        let mut ui = ShopUiState {
            mode: ShopMode::Buy,
            selected: 0,
        };

        player.stats.gold = 50;
        reduce(&mut state, &mut player, &mut ui, Confirm);

        assert_eq!(player.stats.gold, 40);
        assert_eq!(player.inventory.len(), 1);
    }

    #[test]
    fn sell_mode_confirm_sells_item() {
        let (mut state, mut player) = make_shop_state(vec![]);
        let mut ui = ShopUiState {
            mode: ShopMode::Sell,
            selected: 0,
        };
        player.inventory.push(make_item("potion", 20));

        reduce(&mut state, &mut player, &mut ui, Confirm);

        assert!(player.inventory.is_empty());
        assert_eq!(player.stats.gold, 60);
    }

    #[test]
    fn back_in_buy_or_sell_returns_to_select() {
        let (mut state, mut player) = make_shop_state(vec![make_item("potion", 10)]);
        let mut ui = ShopUiState {
            mode: ShopMode::Buy,
            selected: 2,
        };

        reduce(&mut state, &mut player, &mut ui, Back);
        assert!(matches!(ui.mode, ShopMode::Select));
        assert_eq!(ui.selected, 0);
    }

    #[test]
    fn scroll_for_selection_is_derived() {
        assert_eq!(scroll_for_selection(0, 20), 0);
        assert_eq!(scroll_for_selection(7, 20), 0);
        assert_eq!(scroll_for_selection(8, 20), 1);
        assert_eq!(scroll_for_selection(19, 20), 12);
    }
}

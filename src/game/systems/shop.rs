use wipi::event::KeyCode;

use crate::game::{self, GameState, PlayerEvent, PlayerIntent, PlayerState, ShopMode};

const VISIBLE_ITEMS: usize = 8;

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

pub fn reduce(state: &mut GameState, player: &mut PlayerState, intent: ShopIntent) {
    let GameState::Shop(ref mut shop_state) = *state else {
        return;
    };

    match shop_state.mode {
        ShopMode::Select => match intent {
            ShopIntent::MoveUp => {
                if shop_state.selected > 0 {
                    shop_state.selected -= 1;
                    if shop_state.selected < shop_state.scroll {
                        shop_state.scroll = shop_state.selected;
                    }
                }
            }
            ShopIntent::MoveDown => {
                if shop_state.selected + 1 < 2 {
                    shop_state.selected += 1;
                    if shop_state.selected >= shop_state.scroll + 2 {
                        shop_state.scroll = shop_state.selected - 2 + 1;
                    }
                }
            }
            ShopIntent::Confirm => {
                shop_state.mode = if shop_state.selected == 0 {
                    ShopMode::Buy
                } else {
                    ShopMode::Sell
                };
                shop_state.selected = 0;
                shop_state.scroll = 0;
            }
            ShopIntent::Back => *state = GameState::Explore,
        },
        ShopMode::Buy => match intent {
            ShopIntent::MoveUp => {
                if shop_state.selected > 0 {
                    shop_state.selected -= 1;
                    if shop_state.selected < shop_state.scroll {
                        shop_state.scroll = shop_state.selected;
                    }
                }
            }
            ShopIntent::MoveDown => {
                if shop_state.selected + 1 < shop_state.items.len() {
                    shop_state.selected += 1;
                    if shop_state.selected >= shop_state.scroll + VISIBLE_ITEMS {
                        shop_state.scroll = shop_state.selected - VISIBLE_ITEMS + 1;
                    }
                }
            }
            ShopIntent::Confirm => {
                if let Some(item) = shop_state.items.get(shop_state.selected).cloned()
                    && player.stats.gold >= item.price
                {
                    let _ = game::player::reduce(player, PlayerIntent::AddGold(-item.price));
                    let _ = game::player::reduce(player, PlayerIntent::AddItem(item));
                }
            }
            ShopIntent::Back => {
                shop_state.mode = ShopMode::Select;
                shop_state.selected = 0;
                shop_state.scroll = 0;
            }
        },
        ShopMode::Sell => match intent {
            ShopIntent::MoveUp => {
                if shop_state.selected > 0 {
                    shop_state.selected -= 1;
                    if shop_state.selected < shop_state.scroll {
                        shop_state.scroll = shop_state.selected;
                    }
                }
            }
            ShopIntent::MoveDown => {
                if shop_state.selected + 1 < player.inventory.len() {
                    shop_state.selected += 1;
                    if shop_state.selected >= shop_state.scroll + VISIBLE_ITEMS {
                        shop_state.scroll = shop_state.selected - VISIBLE_ITEMS + 1;
                    }
                }
            }
            ShopIntent::Confirm => {
                let event =
                    game::player::reduce(player, PlayerIntent::RemoveItemAt(shop_state.selected));
                if let PlayerEvent::ItemRemoved(Some(item)) = event {
                    let _ = game::player::reduce(player, PlayerIntent::AddGold(item.price / 2));

                    let inv_len = player.inventory.len();
                    if shop_state.selected >= inv_len && shop_state.selected > 0 {
                        shop_state.selected -= 1;
                    }
                    if shop_state.scroll > 0
                        && shop_state.scroll >= inv_len.saturating_sub(VISIBLE_ITEMS - 1)
                    {
                        shop_state.scroll = inv_len.saturating_sub(VISIBLE_ITEMS);
                    }
                }
            }
            ShopIntent::Back => {
                shop_state.mode = ShopMode::Select;
                shop_state.selected = 0;
                shop_state.scroll = 0;
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::ShopIntent::{Back, Confirm, MoveDown, MoveUp};
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

    fn make_shop_state(items: Vec<Item>, mode: ShopMode) -> (GameState, PlayerState) {
        let shop = Shop {
            id: String::from("shop"),
            name: String::from("Shop"),
            items: items.iter().map(|item| item.id.clone()).collect(),
        };
        let mut shop_state = ShopState::new(shop, items);
        shop_state.mode = mode;

        (
            GameState::Shop(shop_state),
            PlayerState::new(String::from("Hero"), "village"),
        )
    }

    fn shop_state(state: &GameState) -> &ShopState {
        let GameState::Shop(shop_state) = state else {
            panic!("expected shop state")
        };
        shop_state
    }

    fn shop_state_mut(state: &mut GameState) -> &mut ShopState {
        let GameState::Shop(shop_state) = state else {
            panic!("expected shop state")
        };
        shop_state
    }

    #[test]
    fn select_mode_move_down_and_up_switches_between_two_options() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Select);

        reduce(&mut state, &mut player, MoveDown);
        assert_eq!(shop_state(&state).selected, 1);

        reduce(&mut state, &mut player, MoveUp);
        assert_eq!(shop_state(&state).selected, 0);
    }

    #[test]
    fn select_mode_move_down_clamps_at_sell_option() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Select);

        reduce(&mut state, &mut player, MoveDown);
        reduce(&mut state, &mut player, MoveDown);

        assert_eq!(shop_state(&state).selected, 1);
    }

    #[test]
    fn select_mode_move_up_clamps_at_buy_option() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Select);

        reduce(&mut state, &mut player, MoveUp);

        assert_eq!(shop_state(&state).selected, 0);
    }

    #[test]
    fn select_mode_confirm_buy_sets_buy_mode_and_resets_selection() {
        let (mut state, mut player) =
            make_shop_state(vec![make_item("potion", 10)], ShopMode::Select);
        {
            let shop_state = shop_state_mut(&mut state);
            shop_state.selected = 0;
            shop_state.scroll = 1;
        }

        reduce(&mut state, &mut player, Confirm);

        let shop_state = shop_state(&state);
        assert_eq!(shop_state.mode, ShopMode::Buy);
        assert_eq!(shop_state.selected, 0);
        assert_eq!(shop_state.scroll, 0);
    }

    #[test]
    fn select_mode_confirm_sell_sets_sell_mode_and_resets_selection() {
        let (mut state, mut player) =
            make_shop_state(vec![make_item("potion", 10)], ShopMode::Select);
        {
            let shop_state = shop_state_mut(&mut state);
            shop_state.selected = 1;
            shop_state.scroll = 1;
        }

        reduce(&mut state, &mut player, Confirm);

        let shop_state = shop_state(&state);
        assert_eq!(shop_state.mode, ShopMode::Sell);
        assert_eq!(shop_state.selected, 0);
        assert_eq!(shop_state.scroll, 0);
    }

    #[test]
    fn select_mode_back_returns_to_explore() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Select);

        reduce(&mut state, &mut player, Back);

        assert!(matches!(state, GameState::Explore));
    }

    #[test]
    fn buy_mode_move_up_down_navigates_shop_items_and_clamps() {
        let (mut state, mut player) =
            make_shop_state(vec![make_item("a", 10), make_item("b", 20)], ShopMode::Buy);

        reduce(&mut state, &mut player, MoveDown);
        reduce(&mut state, &mut player, MoveDown);
        assert_eq!(shop_state(&state).selected, 1);

        reduce(&mut state, &mut player, MoveUp);
        reduce(&mut state, &mut player, MoveUp);
        assert_eq!(shop_state(&state).selected, 0);
    }

    #[test]
    fn buy_mode_confirm_buys_item_when_gold_is_enough() {
        let (mut state, mut player) = make_shop_state(vec![make_item("potion", 30)], ShopMode::Buy);
        player.stats.gold = 100;

        reduce(&mut state, &mut player, Confirm);

        assert_eq!(player.stats.gold, 70);
        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.inventory[0].id, "potion");
    }

    #[test]
    fn buy_mode_confirm_does_nothing_when_gold_is_not_enough() {
        let (mut state, mut player) = make_shop_state(vec![make_item("potion", 80)], ShopMode::Buy);
        player.stats.gold = 10;

        reduce(&mut state, &mut player, Confirm);

        assert_eq!(player.stats.gold, 10);
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn buy_mode_back_returns_to_select_and_resets_position() {
        let (mut state, mut player) = make_shop_state(vec![make_item("potion", 20)], ShopMode::Buy);
        {
            let shop_state = shop_state_mut(&mut state);
            shop_state.selected = 1;
            shop_state.scroll = 1;
        }

        reduce(&mut state, &mut player, Back);

        let shop_state = shop_state(&state);
        assert_eq!(shop_state.mode, ShopMode::Select);
        assert_eq!(shop_state.selected, 0);
        assert_eq!(shop_state.scroll, 0);
    }

    #[test]
    fn sell_mode_move_up_down_navigates_inventory_and_clamps() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Sell);
        player.inventory = vec![make_item("a", 20), make_item("b", 40)];

        reduce(&mut state, &mut player, MoveDown);
        reduce(&mut state, &mut player, MoveDown);
        assert_eq!(shop_state(&state).selected, 1);

        reduce(&mut state, &mut player, MoveUp);
        reduce(&mut state, &mut player, MoveUp);
        assert_eq!(shop_state(&state).selected, 0);
    }

    #[test]
    fn sell_mode_confirm_sells_item_and_adds_half_price_gold() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Sell);
        player.stats.gold = 10;
        player.inventory = vec![make_item("gem", 90)];

        reduce(&mut state, &mut player, Confirm);

        assert!(player.inventory.is_empty());
        assert_eq!(player.stats.gold, 55);
    }

    #[test]
    fn sell_mode_confirm_adjusts_selected_when_last_item_removed() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Sell);
        player.inventory = vec![make_item("first", 20), make_item("last", 30)];
        shop_state_mut(&mut state).selected = 1;

        reduce(&mut state, &mut player, Confirm);

        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.inventory[0].id, "first");
        assert_eq!(shop_state(&state).selected, 0);
    }

    #[test]
    fn sell_mode_back_returns_to_select_and_resets_position() {
        let (mut state, mut player) = make_shop_state(vec![], ShopMode::Sell);
        {
            let shop_state = shop_state_mut(&mut state);
            shop_state.selected = 1;
            shop_state.scroll = 1;
        }

        reduce(&mut state, &mut player, Back);

        let shop_state = shop_state(&state);
        assert_eq!(shop_state.mode, ShopMode::Select);
        assert_eq!(shop_state.selected, 0);
        assert_eq!(shop_state.scroll, 0);
    }
}

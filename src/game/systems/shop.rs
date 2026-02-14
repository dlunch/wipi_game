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

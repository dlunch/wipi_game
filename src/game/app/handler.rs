use wipi::framebuffer::Framebuffer;

use crate::data::Skill;
use crate::game::{
    self, CombatIntent, CombatState, DialogIntent, GameData, GameState, InventoryIntent,
    InventoryState, MenuAction, MenuIntent, MenuState, MovementState, PauseMenuIntent, PlayerState,
    ShopIntent, ShopMode, has_save_data, save_game,
};

use super::{ExploreIntent, update};

pub(super) fn handle_menu_input(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
    intent: MenuIntent,
) {
    let GameState::Menu(ref mut menu) = *state else {
        return;
    };

    match intent {
        MenuIntent::MoveUp => menu.move_up(),
        MenuIntent::MoveDown => menu.move_down(),
        MenuIntent::Select => {
            let action = if menu.has_save {
                match menu.selected {
                    0 => MenuAction::NewGame,
                    1 => MenuAction::Continue,
                    _ => MenuAction::Exit,
                }
            } else {
                match menu.selected {
                    0 => MenuAction::NewGame,
                    _ => MenuAction::Exit,
                }
            };

            match action {
                MenuAction::NewGame => update::start_new_game(state, player, combat, data),
                MenuAction::Continue => update::continue_game(state, player, combat, data),
                MenuAction::Exit => wipi::kernel::exit(0),
            }
        }
    }
}

pub(super) fn handle_explore_input(
    state: &mut GameState,
    movement: &mut MovementState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
    intent: ExploreIntent,
) {
    match intent {
        ExploreIntent::MoveDirection(key) => {
            game::movement::on_direction_pressed(movement, key);
        }
        ExploreIntent::TryNpcInteract => {
            let facing = player.facing;
            if let Some(new_state) =
                game::npc::reduce(player, data, game::NpcIntent::Interact { facing })
            {
                *state = new_state;
            }
        }
        ExploreIntent::Attack => {
            if matches!(*state, GameState::Dialog(_)) {
                return;
            }
            if let game::CombatEvent::Attack(Some(reward)) = game::combat::reduce(
                combat,
                CombatIntent::PlayerAttack {
                    player_x: player.x,
                    player_y: player.y,
                    player_atk: player.total_atk(),
                    facing: player.facing,
                },
            ) {
                player.stats.add_exp(reward.exp);
                player.stats.gold += reward.gold;
                game::quest::on_enemy_killed(player, data, &reward.enemy_id);
            }
        }
        ExploreIntent::Skill1 => update::use_skill(player, combat, data, 0, &Skill::FIREBALL),
        ExploreIntent::Skill2 => update::use_skill(player, combat, data, 1, &Skill::HEAL),
        ExploreIntent::Skill3 => update::use_skill(player, combat, data, 2, &Skill::SPIN_ATTACK),
        ExploreIntent::Pause => *state = GameState::PauseMenu(0),
        ExploreIntent::BackToMenu => {
            let _ = save_game(player);
            *state = GameState::Menu(MenuState {
                selected: 0,
                has_save: has_save_data(),
            });
        }
    }
}

pub(super) fn handle_inventory_input(
    state: &mut GameState,
    player: &mut PlayerState,
    inventory_state: &mut InventoryState,
    intent: InventoryIntent,
) {
    match intent {
        InventoryIntent::MoveUp => inventory_state.move_up(),
        InventoryIntent::MoveDown => {
            let fb = Framebuffer::screen_framebuffer();
            let visible = ((fb.height() as i32 - 50) / 14).max(1) as usize;
            inventory_state.move_down(player.inventory.len(), visible);
        }
        InventoryIntent::UseSelected => {
            let _ = game::player::reduce(
                player,
                game::PlayerIntent::UseItem {
                    index: inventory_state.selected,
                },
            );
        }
        InventoryIntent::Back => *state = GameState::Explore,
    }
}

pub(super) fn handle_dialog_input(
    state: &mut GameState,
    player: &mut PlayerState,
    data: &GameData,
    intent: DialogIntent,
) {
    match intent {
        DialogIntent::Confirm => {
            if let GameState::Dialog(ref dialog_state) = *state
                && let Some(action) = dialog_state.current_action().cloned()
                && let Some(new_state) = game::npc::reduce(
                    player,
                    data,
                    game::NpcIntent::ProcessDialogAction { action: &action },
                )
            {
                *state = new_state;
            }

            if matches!(*state, GameState::Shop(_)) {
                return;
            }

            if let GameState::Dialog(ref mut dialog_state) = *state
                && !dialog_state.advance()
            {
                *state = GameState::Explore;
            }
        }
        DialogIntent::Back => *state = GameState::Explore,
    }
}

pub(super) fn handle_shop_input(
    state: &mut GameState,
    player: &mut PlayerState,
    intent: ShopIntent,
) {
    const VISIBLE_ITEMS: usize = 8;

    let GameState::Shop(ref mut shop_state) = *state else {
        return;
    };

    match shop_state.mode {
        ShopMode::Select => match intent {
            ShopIntent::MoveUp => shop_state.move_up(),
            ShopIntent::MoveDown => shop_state.move_down(2, 2),
            ShopIntent::Confirm => {
                shop_state.mode = if shop_state.selected == 0 {
                    ShopMode::Buy
                } else {
                    ShopMode::Sell
                };
                shop_state.reset_selection();
            }
            ShopIntent::Back => *state = GameState::Explore,
        },
        ShopMode::Buy => match intent {
            ShopIntent::MoveUp => shop_state.move_up(),
            ShopIntent::MoveDown => shop_state.move_down(shop_state.items.len(), VISIBLE_ITEMS),
            ShopIntent::Confirm => {
                if let Some(item) = shop_state.items.get(shop_state.selected).cloned()
                    && player.stats.gold >= item.price
                {
                    player.stats.gold -= item.price;
                    player.add_item(item);
                }
            }
            ShopIntent::Back => {
                shop_state.mode = ShopMode::Select;
                shop_state.reset_selection();
            }
        },
        ShopMode::Sell => match intent {
            ShopIntent::MoveUp => shop_state.move_up(),
            ShopIntent::MoveDown => shop_state.move_down(player.inventory.len(), VISIBLE_ITEMS),
            ShopIntent::Confirm => {
                if let Some(item) = player.remove_item_at(shop_state.selected) {
                    player.stats.gold += item.price / 2;

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
                shop_state.reset_selection();
            }
        },
    }
}

pub(super) fn handle_pause_menu_input(
    state: &mut GameState,
    player: &PlayerState,
    inventory_state: &mut InventoryState,
    intent: PauseMenuIntent,
) {
    let GameState::PauseMenu(ref mut selected) = *state else {
        return;
    };

    match intent {
        PauseMenuIntent::MoveUp if *selected > 0 => *selected -= 1,
        PauseMenuIntent::MoveDown if *selected < 3 => *selected += 1,
        PauseMenuIntent::Select => match *selected {
            0 => {
                *inventory_state = InventoryState::default();
                *state = GameState::Inventory;
            }
            1 => *state = GameState::Stats,
            2 => *state = GameState::QuestLog,
            3 => {
                let _ = save_game(player);
                *state = GameState::Explore;
            }
            _ => {}
        },
        PauseMenuIntent::Back => *state = GameState::Explore,
        _ => {}
    }
}

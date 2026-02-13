use wipi::framebuffer::Framebuffer;

use crate::data::Skill;
use crate::game::{
    self, CombatIntent, DialogIntent, GameState, InventoryIntent, InventoryState, MenuAction,
    MenuIntent, MenuState, PauseMenuIntent, ShopIntent, ShopMode, has_save_data, save_game,
};

use super::{ExploreIntent, RpgGame, update};

pub(super) fn handle_menu_input(game: &mut RpgGame, intent: MenuIntent) {
    let GameState::Menu(ref mut menu) = game.state else {
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
                MenuAction::NewGame => update::start_new_game(game),
                MenuAction::Continue => update::continue_game(game),
                MenuAction::Exit => wipi::kernel::exit(0),
            }
        }
    }
}

pub(super) fn handle_explore_input(game: &mut RpgGame, intent: ExploreIntent) {
    match intent {
        ExploreIntent::MoveDirection(key) => {
            game::movement::on_direction_pressed(&mut game.movement, key);
        }
        ExploreIntent::TryNpcInteract => {
            try_interact_with_npc(game);
        }
        ExploreIntent::Attack => {
            if matches!(game.state, GameState::Dialog(_)) {
                return;
            }
            if let game::CombatEvent::Attack(Some(reward)) = game::combat::reduce(
                &mut game.combat,
                CombatIntent::PlayerAttack {
                    player_x: game.player.x,
                    player_y: game.player.y,
                    player_atk: game.player.total_atk(),
                    facing: game.player.facing,
                },
            ) {
                game.player.stats.add_exp(reward.exp);
                game.player.stats.gold += reward.gold;
                game::quest::on_enemy_killed(&mut game.player, &game.data, &reward.enemy_id);
            }
        }
        ExploreIntent::Skill1 => update::use_skill(game, 0, &Skill::FIREBALL),
        ExploreIntent::Skill2 => update::use_skill(game, 1, &Skill::HEAL),
        ExploreIntent::Skill3 => update::use_skill(game, 2, &Skill::SPIN_ATTACK),
        ExploreIntent::Pause => game.state = GameState::PauseMenu(0),
        ExploreIntent::BackToMenu => {
            let _ = save_game(&game.player);
            game.state = GameState::Menu(MenuState {
                selected: 0,
                has_save: has_save_data(),
            });
        }
    }
}

pub(super) fn handle_inventory_input(game: &mut RpgGame, intent: InventoryIntent) {
    match intent {
        InventoryIntent::MoveUp => game.inventory_state.move_up(),
        InventoryIntent::MoveDown => {
            let fb = Framebuffer::screen_framebuffer();
            let visible = ((fb.height() as i32 - 50) / 14).max(1) as usize;
            game.inventory_state
                .move_down(game.player.inventory.len(), visible);
        }
        InventoryIntent::UseSelected => {
            let _ = game::player::reduce(
                &mut game.player,
                game::PlayerIntent::UseItem {
                    index: game.inventory_state.selected,
                },
            );
        }
        InventoryIntent::Back => game.state = GameState::Explore,
    }
}

pub(super) fn handle_dialog_input(game: &mut RpgGame, intent: DialogIntent) {
    match intent {
        DialogIntent::Confirm => {
            process_dialog_action(game);

            if matches!(game.state, GameState::Shop(_)) {
                return;
            }

            if let GameState::Dialog(ref mut state) = game.state
                && !state.advance()
            {
                game.state = GameState::Explore;
            }
        }
        DialogIntent::Back => game.state = GameState::Explore,
    }
}

pub(super) fn handle_shop_input(game: &mut RpgGame, intent: ShopIntent) {
    const VISIBLE_ITEMS: usize = 8;

    let GameState::Shop(ref mut state) = game.state else {
        return;
    };

    match state.mode {
        ShopMode::Select => match intent {
            ShopIntent::MoveUp => state.move_up(),
            ShopIntent::MoveDown => state.move_down(2, 2),
            ShopIntent::Confirm => {
                state.mode = if state.selected == 0 {
                    ShopMode::Buy
                } else {
                    ShopMode::Sell
                };
                state.reset_selection();
            }
            ShopIntent::Back => game.state = GameState::Explore,
        },
        ShopMode::Buy => match intent {
            ShopIntent::MoveUp => state.move_up(),
            ShopIntent::MoveDown => state.move_down(state.items.len(), VISIBLE_ITEMS),
            ShopIntent::Confirm => {
                if let Some(item) = state.items.get(state.selected).cloned()
                    && game.player.stats.gold >= item.price
                {
                    game.player.stats.gold -= item.price;
                    game.player.add_item(item);
                }
            }
            ShopIntent::Back => {
                state.mode = ShopMode::Select;
                state.reset_selection();
            }
        },
        ShopMode::Sell => match intent {
            ShopIntent::MoveUp => state.move_up(),
            ShopIntent::MoveDown => state.move_down(game.player.inventory.len(), VISIBLE_ITEMS),
            ShopIntent::Confirm => {
                if let Some(item) = game.player.remove_item_at(state.selected) {
                    game.player.stats.gold += item.price / 2;

                    let inv_len = game.player.inventory.len();
                    if state.selected >= inv_len && state.selected > 0 {
                        state.selected -= 1;
                    }
                    if state.scroll > 0
                        && state.scroll >= inv_len.saturating_sub(VISIBLE_ITEMS - 1)
                    {
                        state.scroll = inv_len.saturating_sub(VISIBLE_ITEMS);
                    }
                }
            }
            ShopIntent::Back => {
                state.mode = ShopMode::Select;
                state.reset_selection();
            }
        },
    }
}

pub(super) fn handle_pause_menu_input(game: &mut RpgGame, intent: PauseMenuIntent) {
    let GameState::PauseMenu(ref mut selected) = game.state else {
        return;
    };

    match intent {
        PauseMenuIntent::MoveUp if *selected > 0 => *selected -= 1,
        PauseMenuIntent::MoveDown if *selected < 3 => *selected += 1,
        PauseMenuIntent::Select => match *selected {
            0 => {
                game.inventory_state = InventoryState::default();
                game.state = GameState::Inventory;
            }
            1 => game.state = GameState::Stats,
            2 => game.state = GameState::QuestLog,
            3 => {
                let _ = save_game(&game.player);
                game.state = GameState::Explore;
            }
            _ => {}
        },
        PauseMenuIntent::Back => game.state = GameState::Explore,
        _ => {}
    }
}

pub(super) fn try_interact_with_npc(game: &mut RpgGame) {
    let facing = game.player.facing;
    if let Some(new_state) = game::npc::reduce(
        &mut game.player,
        &game.data,
        game::NpcIntent::Interact { facing },
    ) {
        game.state = new_state;
    }
}

pub(super) fn process_dialog_action(game: &mut RpgGame) {
    let GameState::Dialog(ref state) = game.state else {
        return;
    };

    let Some(action) = state.current_action().cloned() else {
        return;
    };

    if let Some(new_state) = game::npc::reduce(
        &mut game.player,
        &game.data,
        game::NpcIntent::ProcessDialogAction { action: &action },
    ) {
        game.state = new_state;
    }
}

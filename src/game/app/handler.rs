use crate::data::Skill;
use crate::game::{
    self, CombatIntent, CombatState, DialogIntent, GameData, GameState, InventoryIntent,
    InventoryState, MenuAction, MenuEvent, MenuIntent, MenuState, MovementState, PauseMenuIntent,
    PlayerState, ShopIntent, has_save_data, save_game,
};

use super::{ExploreIntent, update};

pub(super) fn handle_menu_input(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
    intent: MenuIntent,
) {
    if let MenuEvent::Action(action) = game::menu::reduce(state, intent) {
        match action {
            MenuAction::NewGame => update::start_new_game(state, player, combat, data),
            MenuAction::Continue => update::continue_game(state, player, combat, data),
            MenuAction::Exit => wipi::kernel::exit(0),
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
    let is_peaceful = data
        .find_map(&player.current_map_id)
        .is_some_and(|m| m.peaceful);

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
        ExploreIntent::Attack if !is_peaceful => {
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
        ExploreIntent::Skill1 if !is_peaceful => {
            update::use_skill(player, combat, data, 0, &Skill::FIREBALL)
        }
        ExploreIntent::Skill2 if !is_peaceful => {
            update::use_skill(player, combat, data, 1, &Skill::HEAL)
        }
        ExploreIntent::Skill3 if !is_peaceful => {
            update::use_skill(player, combat, data, 2, &Skill::SPIN_ATTACK)
        }
        ExploreIntent::Attack
        | ExploreIntent::Skill1
        | ExploreIntent::Skill2
        | ExploreIntent::Skill3 => {}
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
    game::inventory::reduce(state, player, inventory_state, intent);
}

pub(super) fn handle_dialog_input(
    state: &mut GameState,
    player: &mut PlayerState,
    data: &GameData,
    intent: DialogIntent,
) {
    game::dialog::reduce(state, player, data, intent);
}

pub(super) fn handle_shop_input(
    state: &mut GameState,
    player: &mut PlayerState,
    intent: ShopIntent,
) {
    game::shop::reduce(state, player, intent);
}

pub(super) fn handle_pause_menu_input(
    state: &mut GameState,
    player: &PlayerState,
    inventory_state: &mut InventoryState,
    intent: PauseMenuIntent,
) {
    game::menu::reduce_pause(state, player, inventory_state, intent);
}

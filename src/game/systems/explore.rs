use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::data::Skill;
use crate::game::{
    self, CombatIntent, CombatState, GameData, GameState, MenuState, MovementState, PlayerIntent,
    PlayerState, has_save_data, save_game, update,
};

#[derive(Debug, Clone, Copy)]
pub enum ExploreIntent {
    MoveDirection(KeyCode),
    TryNpcInteract,
    Attack,
    Skill1,
    Skill2,
    Skill3,
    Pause,
    BackToMenu,
}

impl ExploreIntent {
    pub fn intent_for_key(key: KeyCode) -> Vec<ExploreIntent> {
        let mut intents = Vec::new();
        match key {
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                intents.push(ExploreIntent::MoveDirection(key));
            }
            KeyCode::Ok => {
                intents.push(ExploreIntent::TryNpcInteract);
                intents.push(ExploreIntent::Attack);
            }
            KeyCode::Key1 => intents.push(ExploreIntent::Skill1),
            KeyCode::Key2 => intents.push(ExploreIntent::Skill2),
            KeyCode::Key3 => intents.push(ExploreIntent::Skill3),
            KeyCode::Key0 => intents.push(ExploreIntent::Pause),
            KeyCode::Back => intents.push(ExploreIntent::BackToMenu),
            _ => {}
        }

        intents
    }
}

pub fn reduce(
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
                let _ = game::player::reduce(player, PlayerIntent::AddExp(reward.exp));
                let _ = game::player::reduce(player, PlayerIntent::AddGold(reward.gold));
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

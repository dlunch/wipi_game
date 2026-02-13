use alloc::format;
use alloc::string::String;

use crate::data::{Skill, Tile};
use crate::game::{
    self, check_tile_event, has_save_data, load_game, CombatIntent, CombatState, DialogState,
    GameData, GameState, MenuState, MovementState, PlayerEffect, PlayerIntent, PlayerState,
    TileEvent,
};

const MP_REGEN_INTERVAL: u32 = 60;

pub(super) fn update_loading(state: &mut GameState, data: &mut GameData) {
    let GameState::Loading(step) = *state else {
        return;
    };

    match data.load_step(step) {
        Ok(true) => {
            *state = GameState::Menu(MenuState {
                selected: 0,
                has_save: has_save_data(),
            });
        }
        Ok(false) => {
            *state = GameState::Loading(step + 1);
        }
        Err(e) => {
            *state = GameState::Error(format!("Load error: {}", e));
        }
    }
}

pub(super) fn update_movement(
    state: &GameState,
    movement: &mut MovementState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    if !matches!(state, GameState::Explore) {
        return;
    }

    let map_id = player.current_map_id.clone();
    let Some(map) = data.find_map(&map_id) else {
        return;
    };

    let moved = game::movement::tick(movement, player, map, combat, &data.npcs);

    if moved {
        check_tile_events(player, combat, data);
    }
}

pub(super) fn update_combat(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    if !matches!(state, GameState::Explore) {
        return;
    }

    let _ = game::player::reduce(player, PlayerIntent::UpdateCooldowns);

    player.mp_regen_timer += 1;
    if player.mp_regen_timer >= MP_REGEN_INTERVAL {
        player.mp_regen_timer = 0;
        let _ = game::player::reduce(player, PlayerIntent::RecoverMp(1));
    }

    let player_x = player.x;
    let player_y = player.y;
    let player_def = player.total_def();
    let map_id = player.current_map_id.clone();

    if let Some(map) = data.find_map(&map_id) {
        let game::CombatEvent::Tick(result) = game::combat::reduce(
            combat,
            CombatIntent::Tick {
                player_x,
                player_y,
                player_def,
                map,
                enemy_data: &data.enemies,
            },
        ) else {
            return;
        };

        if result.damage_taken > 0
            && matches!(
                game::player::reduce(player, PlayerIntent::TakeDamage(result.damage_taken)),
                game::PlayerEvent::Died
            )
        {
            *state = GameState::GameOver;
        }
    }
}

pub(super) fn use_skill(
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
    slot: usize,
    skill: &Skill,
) {
    if !game::player::can_use_skill(player, slot, skill.mp_cost) {
        return;
    }

    let game::CombatEvent::Skill(result) = game::combat::reduce(
        combat,
        CombatIntent::UseSkill {
            skill,
            player_x: player.x,
            player_y: player.y,
            player_atk: player.total_atk(),
            facing: player.facing,
        },
    ) else {
        return;
    };

    let _ = game::player::reduce(
        player,
        PlayerIntent::UseSkill {
            slot,
            mp_cost: skill.mp_cost,
            cooldown: skill.cooldown,
        },
    );

    for effect in &result.player_effects {
        match effect {
            PlayerEffect::Heal(amount) => {
                let _ = game::player::reduce(player, PlayerIntent::Heal(*amount));
            }
        }
    }

    for kill in result.kills {
        player.stats.add_exp(kill.exp);
        player.stats.gold += kill.gold;
        game::quest::on_enemy_killed(player, data, &kill.enemy_id);
    }
}

pub(super) fn check_tile_events(
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    let event = data
        .find_map(&player.current_map_id)
        .and_then(|map| check_tile_event(map, player));

    let Some(event) = event else {
        return;
    };

    match event {
        TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
            if !target.is_empty() {
                change_map(player, combat, data, &target);
            }
        }
        TileEvent::Treasure => {
            let map_id = player.current_map_id.clone();
            if !player.is_treasure_opened(&map_id, player.x, player.y) {
                if let Some(potion) = data.find_item("potion").cloned() {
                    player.add_item(potion);
                }
                player.open_treasure(&map_id, player.x, player.y);
            }
        }
    }
}

pub(super) fn change_map(
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
    target_id: &str,
) {
    let Some(map) = data.find_map(target_id) else {
        return;
    };

    player.current_map_id = map.id.clone();
    if let Some((x, y)) = map.find_player_start() {
        player.x = x;
        player.y = y;
    }
    let _ = game::combat::reduce(
        combat,
        CombatIntent::SpawnEnemies {
            map,
            enemy_data: &data.enemies,
        },
    );
}

pub(super) fn start_new_game(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    *player = PlayerState::new(String::from("Hero"), "village");

    if let Some(sword) = data.find_item("wooden_sword").cloned() {
        let idx = player.inventory.len();
        player.add_item(sword);
        player.equipped_weapon = Some(idx);
    }
    if let Some(armor) = data.find_item("cloth").cloned() {
        let idx = player.inventory.len();
        player.add_item(armor);
        player.equipped_armor = Some(idx);
    }
    if let Some(potion) = data.find_item("potion").cloned() {
        player.add_item(potion.clone());
        player.add_item(potion);
    }

    if let Some(map) = data.find_map("village") {
        player.spawn_at_map(map);
        let _ = game::combat::reduce(
            combat,
            CombatIntent::SpawnEnemies {
                map,
                enemy_data: &data.enemies,
            },
        );
    }

    if let Some(dialog) = data.find_dialog("dialog_guide") {
        *state = GameState::Dialog(DialogState::new(String::from("마을 안내원"), dialog));
    } else {
        *state = GameState::Explore;
    }
}

pub(super) fn continue_game(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    *player = PlayerState::new(String::from("Hero"), "village");

    match load_game(player) {
        Ok(true) => {
            if data.find_map(&player.current_map_id).is_none() {
                player.current_map_id = String::from("village");
            }
            if let Some(map) = data.find_map(&player.current_map_id) {
                if map.get_tile(player.x, player.y) == Tile::Wall
                    || player.x >= map.width
                    || player.y >= map.height
                {
                    player.spawn_at_map(map);
                }
                let _ = game::combat::reduce(
                    combat,
                    CombatIntent::SpawnEnemies {
                        map,
                        enemy_data: &data.enemies,
                    },
                );
            }
            *state = GameState::Explore;
        }
        Ok(false) => {
            start_new_game(state, player, combat, data);
        }
        Err(_) => {
            start_new_game(state, player, combat, data);
        }
    }
}

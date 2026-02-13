use alloc::format;
use alloc::string::String;

use crate::data::{Skill, Tile};
use crate::game::{
    self, check_tile_event, has_save_data, load_game, CombatIntent, GameState, MenuState,
    PlayerEffect, PlayerIntent, PlayerState, TileEvent,
};

use super::{render, RpgGame};

const MP_REGEN_INTERVAL: u32 = 60;

pub(super) fn update_loading(game: &mut RpgGame) {
    let GameState::Loading(step) = game.state else {
        return;
    };

    render::draw_loading(game, step);

    match game.data.load_step(step) {
        Ok(true) => {
            game.state = GameState::Menu(MenuState {
                selected: 0,
                has_save: has_save_data(),
            });
        }
        Ok(false) => {
            game.state = GameState::Loading(step + 1);
        }
        Err(e) => {
            game.state = GameState::Error(format!("Load error: {}", e));
        }
    }
}

pub(super) fn update_movement(game: &mut RpgGame) {
    if !matches!(game.state, GameState::Explore) {
        return;
    }

    let map_id = game.player.current_map_id.clone();
    let Some(map) = game.data.find_map(&map_id) else {
        return;
    };

    let moved = game::movement::tick(
        &mut game.movement,
        &mut game.player,
        map,
        &game.combat,
        &game.data.npcs,
    );

    if moved {
        check_tile_events(game);
    }
}

pub(super) fn update_combat(game: &mut RpgGame) {
    if !matches!(game.state, GameState::Explore) {
        return;
    }

    let _ = game::player::reduce(&mut game.player, PlayerIntent::UpdateCooldowns);

    game.mp_regen_timer += 1;
    if game.mp_regen_timer >= MP_REGEN_INTERVAL {
        game.mp_regen_timer = 0;
        let _ = game::player::reduce(&mut game.player, PlayerIntent::RecoverMp(1));
    }

    let player_x = game.player.x;
    let player_y = game.player.y;
    let player_def = game.player.total_def();
    let map_id = game.player.current_map_id.clone();

    if let Some(map) = game.data.find_map(&map_id) {
        let game::CombatEvent::Tick(result) = game::combat::reduce(
            &mut game.combat,
            CombatIntent::Tick {
                player_x,
                player_y,
                player_def,
                map,
                enemy_data: &game.data.enemies,
            },
        ) else {
            return;
        };

        if result.damage_taken > 0
            && matches!(
                game::player::reduce(
                    &mut game.player,
                    PlayerIntent::TakeDamage(result.damage_taken)
                ),
                game::PlayerEvent::Died
            )
        {
            game.state = GameState::GameOver;
        }
    }
}

pub(super) fn use_skill(game: &mut RpgGame, slot: usize, skill: &Skill) {
    if !game::player::can_use_skill(&game.player, slot, skill.mp_cost) {
        return;
    }

    let game::CombatEvent::Skill(result) = game::combat::reduce(
        &mut game.combat,
        CombatIntent::UseSkill {
            skill,
            player_x: game.player.x,
            player_y: game.player.y,
            player_atk: game.player.total_atk(),
            facing: game.player.facing,
        },
    ) else {
        return;
    };

    let _ = game::player::reduce(
        &mut game.player,
        PlayerIntent::UseSkill {
            slot,
            mp_cost: skill.mp_cost,
            cooldown: skill.cooldown,
        },
    );

    for effect in &result.player_effects {
        match effect {
            PlayerEffect::Heal(amount) => {
                let _ = game::player::reduce(&mut game.player, PlayerIntent::Heal(*amount));
            }
        }
    }

    for kill in result.kills {
        game.player.stats.add_exp(kill.exp);
        game.player.stats.gold += kill.gold;
        game::quest::on_enemy_killed(&mut game.player, &game.data, &kill.enemy_id);
    }
}

pub(super) fn check_tile_events(game: &mut RpgGame) {
    let event = game
        .current_map()
        .and_then(|map| check_tile_event(map, &game.player));

    let Some(event) = event else {
        return;
    };

    match event {
        TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
            if !target.is_empty() {
                change_map(game, &target);
            }
        }
        TileEvent::Treasure => {
            let map_id = game.player.current_map_id.clone();
            if !game
                .player
                .is_treasure_opened(&map_id, game.player.x, game.player.y)
            {
                if let Some(potion) = game.data.find_item("potion").cloned() {
                    game.player.add_item(potion);
                }
                game.player
                    .open_treasure(&map_id, game.player.x, game.player.y);
            }
        }
    }
}

pub(super) fn change_map(game: &mut RpgGame, target_id: &str) {
    let Some(map) = game.data.find_map(target_id) else {
        return;
    };

    game.player.current_map_id = map.id.clone();
    if let Some((x, y)) = map.find_player_start() {
        game.player.x = x;
        game.player.y = y;
    }
    let _ = game::combat::reduce(
        &mut game.combat,
        CombatIntent::SpawnEnemies {
            map,
            enemy_data: &game.data.enemies,
        },
    );
}

pub(super) fn start_new_game(game: &mut RpgGame) {
    game.player = PlayerState::new(String::from("Hero"), "village");

    if let Some(sword) = game.data.find_item("wooden_sword").cloned() {
        let idx = game.player.inventory.len();
        game.player.add_item(sword);
        game.player.equipped_weapon = Some(idx);
    }
    if let Some(armor) = game.data.find_item("cloth").cloned() {
        let idx = game.player.inventory.len();
        game.player.add_item(armor);
        game.player.equipped_armor = Some(idx);
    }
    if let Some(potion) = game.data.find_item("potion").cloned() {
        game.player.add_item(potion.clone());
        game.player.add_item(potion);
    }

    if let Some(map) = game.data.find_map("village") {
        game.player.spawn_at_map(map);
        let _ = game::combat::reduce(
            &mut game.combat,
            CombatIntent::SpawnEnemies {
                map,
                enemy_data: &game.data.enemies,
            },
        );
    }

    game.state = GameState::Explore;
}

pub(super) fn continue_game(game: &mut RpgGame) {
    game.player = PlayerState::new(String::from("Hero"), "village");

    match load_game(&mut game.player) {
        Ok(true) => {
            if game.data.find_map(&game.player.current_map_id).is_none() {
                game.player.current_map_id = String::from("village");
            }
            if let Some(map) = game.data.find_map(&game.player.current_map_id) {
                if map.get_tile(game.player.x, game.player.y) == Tile::Wall
                    || game.player.x >= map.width
                    || game.player.y >= map.height
                {
                    game.player.spawn_at_map(map);
                }
                let _ = game::combat::reduce(
                    &mut game.combat,
                    CombatIntent::SpawnEnemies {
                        map,
                        enemy_data: &game.data.enemies,
                    },
                );
            }
            game.state = GameState::Explore;
        }
        Ok(false) => {
            start_new_game(game);
        }
        Err(_) => {
            start_new_game(game);
        }
    }
}

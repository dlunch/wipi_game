use alloc::format;
use alloc::string::String;

use crate::data::{Map, Skill, Tile};
use crate::game::{
    self, CombatIntent, CombatState, DialogState, GameData, GameState, MenuState, MovementState,
    PlayerEffect, PlayerIntent, PlayerState, has_save_data, load_game,
};

#[derive(Debug, Clone)]
enum TileEvent {
    Treasure,
    MapExit(String),
    DungeonEntrance(String),
}

fn check_tile_event(map: &Map, player: &PlayerState) -> Option<TileEvent> {
    let tile = map.get_tile(player.x, player.y);

    match tile {
        Tile::Treasure => Some(TileEvent::Treasure),
        Tile::Exit => {
            for (ex, ey, target) in &map.exits {
                if *ex == player.x && *ey == player.y {
                    return Some(TileEvent::MapExit(target.clone()));
                }
            }
            None
        }
        Tile::Dungeon => {
            for (dx, dy, target) in &map.dungeons {
                if *dx == player.x && *dy == player.y {
                    return Some(TileEvent::DungeonEntrance(target.clone()));
                }
            }
            None
        }
        _ => None,
    }
}

pub fn update_loading(state: &mut GameState, data: &mut GameData) {
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

pub fn update_movement(
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

pub fn update_combat(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    if !matches!(state, GameState::Explore) {
        return;
    }

    let _ = game::player::reduce(player, PlayerIntent::UpdateCooldowns);
    let _ = game::player::reduce(player, PlayerIntent::TickMpRegen);

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

pub fn use_skill(
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
        let _ = game::player::reduce(player, PlayerIntent::AddExp(kill.exp));
        let _ = game::player::reduce(player, PlayerIntent::AddGold(kill.gold));
        game::quest::on_enemy_killed(player, data, &kill.enemy_id);
    }
}

pub fn check_tile_events(player: &mut PlayerState, combat: &mut CombatState, data: &GameData) {
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
                    let _ = game::player::reduce(player, PlayerIntent::AddItem(potion));
                }
                let _ = game::player::reduce(
                    player,
                    PlayerIntent::OpenTreasure {
                        map_id,
                        x: player.x,
                        y: player.y,
                    },
                );
            }
        }
    }
}

pub fn change_map(
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
    target_id: &str,
) {
    let Some(map) = data.find_map(target_id) else {
        return;
    };

    let (x, y) = map.find_player_start().unwrap_or((player.x, player.y));
    let _ = game::player::reduce(
        player,
        PlayerIntent::ChangeMap {
            map_id: map.id.clone(),
            x,
            y,
        },
    );
    let _ = game::combat::reduce(
        combat,
        CombatIntent::SpawnEnemies {
            map,
            enemy_data: &data.enemies,
        },
    );
}

pub fn start_new_game(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    *player = PlayerState::new(String::from("Hero"), "village");

    if let Some(sword) = data.find_item("wooden_sword").cloned() {
        let idx = player.inventory.len();
        let _ = game::player::reduce(player, PlayerIntent::AddItem(sword));
        let _ = game::player::reduce(player, PlayerIntent::EquipWeapon(idx));
    }
    if let Some(armor) = data.find_item("cloth").cloned() {
        let idx = player.inventory.len();
        let _ = game::player::reduce(player, PlayerIntent::AddItem(armor));
        let _ = game::player::reduce(player, PlayerIntent::EquipArmor(idx));
    }
    if let Some(potion) = data.find_item("potion").cloned() {
        let _ = game::player::reduce(player, PlayerIntent::AddItem(potion.clone()));
        let _ = game::player::reduce(player, PlayerIntent::AddItem(potion));
    }

    if let Some(map) = data.find_map("village") {
        let (x, y) = map.find_player_start().unwrap_or((player.x, player.y));
        let _ = game::player::reduce(
            player,
            PlayerIntent::ChangeMap {
                map_id: map.id.clone(),
                x,
                y,
            },
        );
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

pub fn continue_game(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) {
    *player = PlayerState::new(String::from("Hero"), "village");

    match load_game(player) {
        Ok(true) => {
            if data.find_map(&player.current_map_id).is_none() {
                let _ = game::player::reduce(
                    player,
                    PlayerIntent::ChangeMap {
                        map_id: String::from("village"),
                        x: player.x,
                        y: player.y,
                    },
                );
            }
            if let Some(map) = data.find_map(&player.current_map_id) {
                if (map.get_tile(player.x, player.y) == Tile::Wall
                    || player.x >= map.width
                    || player.y >= map.height)
                    && let Some((x, y)) = map.find_player_start()
                {
                    let _ = game::player::reduce(player, PlayerIntent::SpawnAtMap { x, y });
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

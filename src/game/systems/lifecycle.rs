use alloc::format;
use alloc::string::String;

use crate::data::Tile;
use crate::game::{
    self, CombatIntent, CombatState, DialogState, GameData, GameState, MenuState, PlayerIntent,
    PlayerState, has_save_data, load_game,
};

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

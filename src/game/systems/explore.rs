use alloc::string::String;
use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::data::{Map, Skill, Tile};
use crate::game::{
    self, CombatIntent, CombatState, GameData, GameState, MenuState, MovementState, PauseMenuState,
    PlayerIntent, PlayerState, has_save_data, save_game,
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
    skill_cooldowns: &mut [u32; 3],
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
                game::quest::reduce(
                    player,
                    data,
                    game::QuestIntent::EnemyKilled {
                        enemy_id: &reward.enemy_id,
                    },
                );
            }
        }
        ExploreIntent::Skill1 if !is_peaceful => game::combat::use_skill_action(
            player,
            skill_cooldowns,
            combat,
            data,
            0,
            &Skill::FIREBALL,
        ),
        ExploreIntent::Skill2 if !is_peaceful => {
            game::combat::use_skill_action(player, skill_cooldowns, combat, data, 1, &Skill::HEAL)
        }
        ExploreIntent::Skill3 if !is_peaceful => game::combat::use_skill_action(
            player,
            skill_cooldowns,
            combat,
            data,
            2,
            &Skill::SPIN_ATTACK,
        ),
        ExploreIntent::Attack
        | ExploreIntent::Skill1
        | ExploreIntent::Skill2
        | ExploreIntent::Skill3 => {}
        ExploreIntent::Pause => *state = GameState::PauseMenu(PauseMenuState::new()),
        ExploreIntent::BackToMenu => {
            let _ = save_game(player);
            *state = GameState::Menu(MenuState::new(has_save_data()));
        }
    }
}

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

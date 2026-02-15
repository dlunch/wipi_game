use alloc::string::String;
use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::data::{Map, Skill, Tile};
use crate::game::{
    self, CombatIntent, CombatState, GameData, GameState, MovementState, PlayerIntent, PlayerState,
    save_game,
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

pub struct ExploreRuntime<'a> {
    pub movement: &'a mut MovementState,
    pub player: &'a mut PlayerState,
    pub skill_cooldowns: &'a mut [u32; 3],
    pub combat: &'a mut CombatState,
}

pub enum ExploreEvent {
    None,
    OpenDialog(crate::game::DialogState),
    OpenShop(crate::game::ShopState),
    EnterPauseMenu,
    EnterMenu,
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
    state: &GameState,
    runtime: ExploreRuntime<'_>,
    data: &GameData,
    intent: ExploreIntent,
) -> ExploreEvent {
    let ExploreRuntime {
        movement,
        player,
        skill_cooldowns,
        combat,
    } = runtime;

    let is_peaceful = data
        .find_map(&player.current_map_id)
        .is_some_and(|m| m.peaceful);

    match intent {
        ExploreIntent::MoveDirection(key) => {
            game::movement::on_direction_pressed(movement, key);
        }
        ExploreIntent::TryNpcInteract => {
            let facing = player.facing;
            if let Some(event) =
                game::npc::reduce(player, data, game::NpcIntent::Interact { facing })
            {
                match event {
                    game::NpcEvent::OpenDialog(dialog_state) => {
                        return ExploreEvent::OpenDialog(dialog_state);
                    }
                    game::NpcEvent::OpenShop(shop_state) => {
                        return ExploreEvent::OpenShop(shop_state);
                    }
                }
            }
        }
        ExploreIntent::Attack if !is_peaceful => {
            if matches!(*state, GameState::Dialog) {
                return ExploreEvent::None;
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
        | ExploreIntent::Skill3 => {
            return ExploreEvent::None;
        }
        ExploreIntent::Pause => {
            return ExploreEvent::EnterPauseMenu;
        }
        ExploreIntent::BackToMenu => {
            let _ = save_game(player);
            return ExploreEvent::EnterMenu;
        }
    }

    ExploreEvent::None
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

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use wipi::event::KeyCode;

    use super::*;
    use crate::data::{Direction, Enemy, Item, ItemKind, Map, Tile};
    use crate::game::combat::FieldEnemy;

    fn make_test_map(
        id: &str,
        width: usize,
        height: usize,
        tiles: Vec<Tile>,
        exits: Vec<(usize, usize, String)>,
        dungeons: Vec<(usize, usize, String)>,
        encounters: Vec<(String, i32)>,
        peaceful: bool,
    ) -> Map {
        Map {
            id: id.into(),
            name: id.into(),
            width,
            height,
            tiles,
            encounters,
            exits,
            dungeons,
            npcs: Vec::new(),
            peaceful,
        }
    }

    fn make_game_data() -> GameData {
        let mut data = GameData::default();

        data.items.push(Item {
            id: "potion".into(),
            name: "Potion".into(),
            kind: ItemKind::Consumable,
            param1: 30,
            param2: 0,
            param3: 0,
            price: 50,
        });

        data.enemies.push(Enemy {
            id: "slime".into(),
            name: "Slime".into(),
            hp: 10,
            atk: 4,
            def: 1,
            exp: 5,
            gold: 2,
        });

        data.maps.push(make_test_map(
            "field",
            3,
            3,
            vec![
                Tile::PlayerStart,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Treasure,
                Tile::Exit,
                Tile::Floor,
                Tile::Dungeon,
                Tile::Floor,
            ],
            vec![(2, 1, "town".into())],
            vec![(1, 2, "cave".into())],
            Vec::new(),
            false,
        ));

        data.maps.push(make_test_map(
            "town",
            3,
            3,
            vec![
                Tile::PlayerStart,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        ));

        data.maps.push(make_test_map(
            "cave",
            3,
            3,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::PlayerStart,
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        ));

        data.maps.push(make_test_map(
            "battlefield",
            3,
            3,
            vec![
                Tile::Enemy,
                Tile::PlayerStart,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
            Vec::new(),
            Vec::new(),
            vec![("slime".into(), 1)],
            false,
        ));

        data.maps.push(make_test_map(
            "safe_zone",
            2,
            2,
            vec![Tile::PlayerStart, Tile::Floor, Tile::Floor, Tile::Floor],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        ));

        data
    }

    fn make_player(map_id: &str, x: usize, y: usize) -> PlayerState {
        let mut player = PlayerState::new(String::from("Tester"), map_id);
        player.x = x;
        player.y = y;
        player
    }

    #[test]
    fn intent_for_key_direction_keys_map_to_move_direction() {
        for key in [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right] {
            let intents = ExploreIntent::intent_for_key(key);
            assert_eq!(intents.len(), 1);
            assert!(matches!(intents.as_slice(), [ExploreIntent::MoveDirection(k)] if *k == key));
        }
    }

    #[test]
    fn intent_for_key_ok_returns_interact_then_attack() {
        let intents = ExploreIntent::intent_for_key(KeyCode::Ok);
        assert!(matches!(
            intents.as_slice(),
            [ExploreIntent::TryNpcInteract, ExploreIntent::Attack]
        ));
    }

    #[test]
    fn intent_for_key_skill_keys_map_to_skills() {
        assert!(matches!(
            ExploreIntent::intent_for_key(KeyCode::Key1).as_slice(),
            [ExploreIntent::Skill1]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(KeyCode::Key2).as_slice(),
            [ExploreIntent::Skill2]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(KeyCode::Key3).as_slice(),
            [ExploreIntent::Skill3]
        ));
    }

    #[test]
    fn intent_for_key_pause_and_back_keys_map_to_menu_intents() {
        assert!(matches!(
            ExploreIntent::intent_for_key(KeyCode::Key0).as_slice(),
            [ExploreIntent::Pause]
        ));
        assert!(matches!(
            ExploreIntent::intent_for_key(KeyCode::Back).as_slice(),
            [ExploreIntent::BackToMenu]
        ));
    }

    #[test]
    fn check_tile_events_treasure_adds_potion_and_records_opened() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 1);
        let mut combat = CombatState::default();

        check_tile_events(&mut player, &mut combat, &data);

        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.inventory[0].id, "potion");
        assert_eq!(player.opened_treasures.len(), 1);
        assert!(player.is_treasure_opened("field", 1, 1));
    }

    #[test]
    fn check_tile_events_already_opened_treasure_has_no_duplicate_rewards() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 1);
        let mut combat = CombatState::default();

        check_tile_events(&mut player, &mut combat, &data);
        check_tile_events(&mut player, &mut combat, &data);

        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.opened_treasures.len(), 1);
    }

    #[test]
    fn check_tile_events_exit_tile_changes_map_when_exit_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 2, 1);
        let mut combat = CombatState::default();

        check_tile_events(&mut player, &mut combat, &data);

        assert_eq!(player.current_map_id, "town");
        assert_eq!(player.x, 0);
        assert_eq!(player.y, 0);
    }

    #[test]
    fn check_tile_events_dungeon_tile_changes_map_when_dungeon_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 2);
        let mut combat = CombatState::default();

        check_tile_events(&mut player, &mut combat, &data);

        assert_eq!(player.current_map_id, "cave");
        assert_eq!(player.x, 2);
        assert_eq!(player.y, 2);
    }

    #[test]
    fn check_tile_events_floor_tile_does_nothing() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 0);
        let mut combat = CombatState::default();

        check_tile_events(&mut player, &mut combat, &data);

        assert_eq!(player.current_map_id, "field");
        assert!(player.inventory.is_empty());
        assert!(player.opened_treasures.is_empty());
        assert!(combat.enemies.is_empty());
    }

    #[test]
    fn change_map_changes_player_map_and_position_and_spawns_enemies() {
        let data = make_game_data();
        let mut player = make_player("field", 0, 0);
        let mut combat = CombatState::default();

        change_map(&mut player, &mut combat, &data, "battlefield");

        assert_eq!(player.current_map_id, "battlefield");
        assert_eq!(player.x, 1);
        assert_eq!(player.y, 0);
        assert_eq!(combat.enemies.len(), 1);
        assert_eq!(combat.enemies[0].x, 0);
        assert_eq!(combat.enemies[0].y, 0);
    }

    #[test]
    fn reduce_pause_switches_to_pause_menu_state() {
        let data = make_game_data();
        let state = GameState::Explore;
        let mut movement = MovementState::default();
        let mut player = make_player("field", 0, 0);
        let mut skill_cooldowns = [0; 3];
        let mut combat = CombatState::default();

        let event = reduce(
            &state,
            ExploreRuntime {
                movement: &mut movement,
                player: &mut player,
                skill_cooldowns: &mut skill_cooldowns,
                combat: &mut combat,
            },
            &data,
            ExploreIntent::Pause,
        );

        assert!(matches!(event, ExploreEvent::EnterPauseMenu));
    }

    #[test]
    fn reduce_attack_has_no_effect_in_peaceful_zone() {
        let data = make_game_data();
        let state = GameState::Explore;
        let mut movement = MovementState::default();
        let mut player = make_player("safe_zone", 0, 0);
        player.facing = Direction::Right;
        let mut skill_cooldowns = [0; 3];
        let mut combat = CombatState::default();
        combat.enemies.push(FieldEnemy::new(
            Enemy {
                id: "slime".into(),
                name: "Slime".into(),
                hp: 10,
                atk: 4,
                def: 1,
                exp: 5,
                gold: 2,
            },
            1,
            0,
        ));

        let event = reduce(
            &state,
            ExploreRuntime {
                movement: &mut movement,
                player: &mut player,
                skill_cooldowns: &mut skill_cooldowns,
                combat: &mut combat,
            },
            &data,
            ExploreIntent::Attack,
        );

        assert!(matches!(event, ExploreEvent::None));
        assert_eq!(combat.enemies[0].hp, 10);
        assert_eq!(combat.player_attack_cooldown, 0);
        assert!(combat.skill_effects.is_empty());
    }
}

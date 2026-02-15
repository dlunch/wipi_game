use wipi::event::KeyCode;

use super::combat::{enemy_at, CombatState};
use crate::data::{Map, Npc};
use crate::game::{self, GameData, GameState, PlayerIntent, PlayerState};

const MOVE_COOLDOWN: u32 = 5;

#[derive(Default)]
pub struct MovementState {
    pub pressed_direction: Option<KeyCode>,
    pub move_cooldown: u32,
}

pub fn update(
    game_state: &GameState,
    state: &mut MovementState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
) -> bool {
    if !matches!(game_state, GameState::Explore) {
        return false;
    }

    let map_id = player.current_map_id.clone();
    let Some(map) = data.find_map(&map_id) else {
        return false;
    };

    tick(state, player, map, combat, &data.npcs)
}

pub fn on_direction_pressed(state: &mut MovementState, key: KeyCode) {
    state.pressed_direction = Some(key);
    state.move_cooldown = 0;
}

pub fn on_key_released(state: &mut MovementState, key: KeyCode) {
    if state.pressed_direction == Some(key) {
        state.pressed_direction = None;
    }
}

fn tick(
    state: &mut MovementState,
    player: &mut PlayerState,
    map: &Map,
    combat: &CombatState,
    npcs: &[Npc],
) -> bool {
    if state.move_cooldown > 0 {
        state.move_cooldown -= 1;
        return false;
    }

    let Some(key) = state.pressed_direction else {
        return false;
    };

    let moved = try_move(player, map, combat, npcs, key);
    state.move_cooldown = MOVE_COOLDOWN;
    moved
}

fn can_move(player: &PlayerState, map: &Map, dx: i32, dy: i32) -> bool {
    let Some(new_x) = player.x.checked_add_signed(dx as isize) else {
        return false;
    };
    let Some(new_y) = player.y.checked_add_signed(dy as isize) else {
        return false;
    };
    map.get_tile(new_x, new_y).is_passable()
}

fn try_move(
    player: &mut PlayerState,
    map: &Map,
    combat: &CombatState,
    npcs: &[Npc],
    key: KeyCode,
) -> bool {
    let (dx, dy) = match key {
        KeyCode::Up => (0, -1),
        KeyCode::Down => (0, 1),
        KeyCode::Left => (-1, 0),
        KeyCode::Right => (1, 0),
        _ => return false,
    };

    let _ = game::player::reduce(player, PlayerIntent::SetFacing { dx, dy });

    if !can_move(player, map, dx, dy) {
        return false;
    }

    let Some(new_x) = player.x.checked_add_signed(dx as isize) else {
        return false;
    };
    let Some(new_y) = player.y.checked_add_signed(dy as isize) else {
        return false;
    };

    if enemy_at(combat, new_x, new_y) {
        return false;
    }

    if npc_at(npcs, &player.current_map_id, new_x, new_y) {
        return false;
    }

    let _ = game::player::reduce(player, PlayerIntent::MoveBy { dx, dy });
    true
}

fn npc_at(npcs: &[Npc], map_id: &str, x: usize, y: usize) -> bool {
    npcs.iter()
        .any(|npc| npc.map_id == map_id && npc.x == x && npc.y == y)
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use wipi::event::KeyCode;

    use super::*;
    use crate::data::{Direction, Enemy, Map, Npc, NpcType, Tile};
    use crate::game::{combat::FieldEnemy, PlayerState};

    fn make_test_map(width: usize, height: usize, tiles: Vec<Tile>) -> Map {
        Map {
            id: String::from("test_map"),
            name: String::from("Test Map"),
            width,
            height,
            tiles,
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: false,
        }
    }

    fn make_player_at(x: usize, y: usize, map_id: &str) -> PlayerState {
        let mut player = PlayerState::new(String::from("Tester"), map_id);
        player.x = x;
        player.y = y;
        player
    }

    fn make_enemy() -> Enemy {
        Enemy {
            id: String::from("slime"),
            name: String::from("Slime"),
            hp: 10,
            atk: 3,
            def: 1,
            exp: 5,
            gold: 2,
        }
    }

    #[test]
    fn on_direction_pressed_sets_direction_and_resets_cooldown() {
        let mut state = MovementState {
            pressed_direction: None,
            move_cooldown: 3,
        };

        on_direction_pressed(&mut state, KeyCode::Left);

        assert!(state.pressed_direction == Some(KeyCode::Left));
        assert!(state.move_cooldown == 0);
    }

    #[test]
    fn on_key_released_clears_when_matching_key() {
        let mut state = MovementState {
            pressed_direction: Some(KeyCode::Right),
            move_cooldown: 0,
        };

        on_key_released(&mut state, KeyCode::Right);

        assert!(state.pressed_direction.is_none());
    }

    #[test]
    fn on_key_released_keeps_direction_when_different_key() {
        let mut state = MovementState {
            pressed_direction: Some(KeyCode::Up),
            move_cooldown: 0,
        };

        on_key_released(&mut state, KeyCode::Down);

        assert!(state.pressed_direction == Some(KeyCode::Up));
    }

    #[test]
    fn can_move_returns_true_on_passable_tile() {
        let map = make_test_map(
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
                Tile::Floor,
            ],
        );
        let player = make_player_at(1, 1, "test_map");

        assert!(can_move(&player, &map, 1, 0));
    }

    #[test]
    fn can_move_returns_false_on_wall() {
        let map = make_test_map(
            3,
            3,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Wall,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
        );
        let player = make_player_at(1, 1, "test_map");

        assert!(!can_move(&player, &map, 1, 0));
    }

    #[test]
    fn can_move_returns_false_out_of_bounds() {
        let map = make_test_map(
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
                Tile::Floor,
            ],
        );
        let player = make_player_at(0, 0, "test_map");

        assert!(!can_move(&player, &map, -1, 0));
    }

    #[test]
    fn tick_returns_false_during_cooldown() {
        let map = make_test_map(
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
                Tile::Floor,
            ],
        );
        let mut player = make_player_at(1, 1, "test_map");
        let combat = CombatState::default();
        let npcs: Vec<Npc> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(KeyCode::Right),
            move_cooldown: 2,
        };

        let moved = tick(&mut state, &mut player, &map, &combat, &npcs);

        assert!(!moved);
        assert!(state.move_cooldown == 1);
        assert!(player.x == 1 && player.y == 1);
    }

    #[test]
    fn tick_returns_false_with_no_pressed_direction() {
        let map = make_test_map(
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
                Tile::Floor,
            ],
        );
        let mut player = make_player_at(1, 1, "test_map");
        let combat = CombatState::default();
        let npcs: Vec<Npc> = Vec::new();
        let mut state = MovementState::default();

        let moved = tick(&mut state, &mut player, &map, &combat, &npcs);

        assert!(!moved);
        assert!(state.move_cooldown == 0);
        assert!(player.x == 1 && player.y == 1);
    }

    #[test]
    fn tick_processes_move_after_cooldown_expires() {
        let map = make_test_map(
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
                Tile::Floor,
            ],
        );
        let mut player = make_player_at(1, 1, "test_map");
        let combat = CombatState::default();
        let npcs: Vec<Npc> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(KeyCode::Right),
            move_cooldown: 1,
        };

        let moved_while_cooling = tick(&mut state, &mut player, &map, &combat, &npcs);
        let moved_after_cooling = tick(&mut state, &mut player, &map, &combat, &npcs);

        assert!(!moved_while_cooling);
        assert!(moved_after_cooling);
        assert!(player.x == 2 && player.y == 1);
        assert!(matches!(player.facing, Direction::Right));
        assert!(state.move_cooldown == MOVE_COOLDOWN);
    }

    #[test]
    fn tick_try_move_blocked_by_wall() {
        let map = make_test_map(
            3,
            3,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Wall,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
        );
        let mut player = make_player_at(1, 1, "test_map");
        let combat = CombatState::default();
        let npcs: Vec<Npc> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(KeyCode::Right),
            move_cooldown: 0,
        };

        let moved = tick(&mut state, &mut player, &map, &combat, &npcs);

        assert!(!moved);
        assert!(player.x == 1 && player.y == 1);
        assert!(matches!(player.facing, Direction::Right));
    }

    #[test]
    fn tick_try_move_blocked_by_enemy() {
        let map = make_test_map(
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
                Tile::Floor,
            ],
        );
        let mut player = make_player_at(1, 1, "test_map");
        let enemy = FieldEnemy::new(make_enemy(), 2, 1);
        let combat = CombatState {
            enemies: vec![enemy],
            ..CombatState::default()
        };
        let npcs: Vec<Npc> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(KeyCode::Right),
            move_cooldown: 0,
        };

        let moved = tick(&mut state, &mut player, &map, &combat, &npcs);

        assert!(!moved);
        assert!(player.x == 1 && player.y == 1);
        assert!(matches!(player.facing, Direction::Right));
    }

    #[test]
    fn tick_try_move_blocked_by_npc() {
        let map = make_test_map(
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
                Tile::Floor,
            ],
        );
        let mut player = make_player_at(1, 1, "test_map");
        let combat = CombatState::default();
        let npcs = vec![Npc {
            id: String::from("npc_1"),
            name: String::from("Guide"),
            map_id: String::from("test_map"),
            x: 2,
            y: 1,
            npc_type: NpcType::Villager,
            dialog_id: String::from("dialog_1"),
            shop_id: None,
        }];
        let mut state = MovementState {
            pressed_direction: Some(KeyCode::Right),
            move_cooldown: 0,
        };

        let moved = tick(&mut state, &mut player, &map, &combat, &npcs);

        assert!(!moved);
        assert!(player.x == 1 && player.y == 1);
        assert!(matches!(player.facing, Direction::Right));
    }
}

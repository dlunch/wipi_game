use crate::data::{Direction, Map};
use crate::game::{MovementState, PlayerState};

const MOVE_COOLDOWN: u32 = 5;

pub struct MovementTickEvent {
    pub next_state: MovementState,
    pub facing: Option<(i32, i32)>,
    pub step: Option<(i32, i32)>,
}

pub fn reduce_tick(
    state: &MovementState,
    player: &PlayerState,
    map: Option<&Map>,
    enemy_positions: &[(usize, usize)],
    npc_positions: &[(usize, usize)],
) -> MovementTickEvent {
    let mut next_state = *state;

    let Some(map) = map else {
        return MovementTickEvent {
            next_state,
            facing: None,
            step: None,
        };
    };

    if state.move_cooldown > 0 {
        next_state.move_cooldown -= 1;
        return MovementTickEvent {
            next_state,
            facing: None,
            step: None,
        };
    }

    let Some(key) = state.pressed_direction else {
        return MovementTickEvent {
            next_state,
            facing: None,
            step: None,
        };
    };

    let (dx, dy) = match key {
        Direction::Up => (0, -1),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    };

    next_state.move_cooldown = MOVE_COOLDOWN;
    let mut step = None;

    if can_move(player, map, dx, dy)
        && let Some(new_x) = player.x.checked_add_signed(dx as isize)
        && let Some(new_y) = player.y.checked_add_signed(dy as isize)
        && !position_occupied(enemy_positions, new_x, new_y)
        && !position_occupied(npc_positions, new_x, new_y)
    {
        step = Some((dx, dy));
    }

    MovementTickEvent {
        next_state,
        facing: Some((dx, dy)),
        step,
    }
}

#[cfg(test)]
fn tick(
    state: &mut MovementState,
    player: &mut PlayerState,
    map: &Map,
    enemy_positions: &[(usize, usize)],
    npc_positions: &[(usize, usize)],
) -> bool {
    let event = reduce_tick(state, player, Some(map), enemy_positions, npc_positions);
    state.apply_tick(player, event)
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

fn position_occupied(positions: &[(usize, usize)], x: usize, y: usize) -> bool {
    positions.iter().any(|(ox, oy)| *ox == x && *oy == y)
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Direction, Map, Tile};
    use crate::game::PlayerState;

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

    #[test]
    fn on_direction_pressed_sets_direction_and_resets_cooldown() {
        let mut state = MovementState {
            pressed_direction: None,
            move_cooldown: 3,
        };

        state.on_direction_pressed(Direction::Left);

        assert!(state.pressed_direction == Some(Direction::Left));
        assert!(state.move_cooldown == 0);
    }

    #[test]
    fn on_key_released_clears_when_matching_key() {
        let mut state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };

        state.on_direction_released(Direction::Right);

        assert!(state.pressed_direction.is_none());
    }

    #[test]
    fn on_key_released_keeps_direction_when_different_key() {
        let mut state = MovementState {
            pressed_direction: Some(Direction::Up),
            move_cooldown: 0,
        };

        state.on_direction_released(Direction::Down);

        assert!(state.pressed_direction == Some(Direction::Up));
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
        let enemy_positions: Vec<(usize, usize)> = Vec::new();
        let npc_positions: Vec<(usize, usize)> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 2,
        };

        let moved = tick(
            &mut state,
            &mut player,
            &map,
            &enemy_positions,
            &npc_positions,
        );

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
        let enemy_positions: Vec<(usize, usize)> = Vec::new();
        let npc_positions: Vec<(usize, usize)> = Vec::new();
        let mut state = MovementState::default();

        let moved = tick(
            &mut state,
            &mut player,
            &map,
            &enemy_positions,
            &npc_positions,
        );

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
        let enemy_positions: Vec<(usize, usize)> = Vec::new();
        let npc_positions: Vec<(usize, usize)> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 1,
        };

        let moved_while_cooling = tick(
            &mut state,
            &mut player,
            &map,
            &enemy_positions,
            &npc_positions,
        );
        let moved_after_cooling = tick(
            &mut state,
            &mut player,
            &map,
            &enemy_positions,
            &npc_positions,
        );

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
        let enemy_positions: Vec<(usize, usize)> = Vec::new();
        let npc_positions: Vec<(usize, usize)> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };

        let moved = tick(
            &mut state,
            &mut player,
            &map,
            &enemy_positions,
            &npc_positions,
        );

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
        let enemy_positions: Vec<(usize, usize)> = vec![(2, 1)];
        let npc_positions: Vec<(usize, usize)> = Vec::new();
        let mut state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };

        let moved = tick(
            &mut state,
            &mut player,
            &map,
            &enemy_positions,
            &npc_positions,
        );

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
        let enemy_positions: Vec<(usize, usize)> = Vec::new();
        let npc_positions: Vec<(usize, usize)> = vec![(2, 1)];
        let mut state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };

        let moved = tick(
            &mut state,
            &mut player,
            &map,
            &enemy_positions,
            &npc_positions,
        );

        assert!(!moved);
        assert!(player.x == 1 && player.y == 1);
        assert!(matches!(player.facing, Direction::Right));
    }
}

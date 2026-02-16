use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::{Direction, Map, Tile};

use crate::game::state::FieldEnemy;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{
    CharacterState, GameData, GameEvent, GameState, MovementEvent, MovementState,
    MovementTickEvent, TileEvent, TransitionEvent,
};

const MOVE_COOLDOWN: u32 = 2;

pub struct MovementUpdateResult {
    pub movement_event: MovementTickEvent,
    pub tile_event: Option<TileEvent>,
    pub map_changed: bool,
}

pub fn resolve_world_tick(
    state: &MovementState,
    player: &CharacterState,
    enemies: &[FieldEnemy],
    data: &GameData,
) -> MovementUpdateResult {
    let map = data.find_map(&player.current_map_id);
    let enemy_positions: Vec<(usize, usize)> = enemies
        .iter()
        .filter(|enemy| enemy.hp > 0)
        .map(|enemy| (enemy.x, enemy.y))
        .collect();
    let npc_positions: Vec<(usize, usize)> = data
        .npcs
        .iter()
        .filter(|npc| npc.map_id == player.current_map_id)
        .map(|npc| (npc.x, npc.y))
        .collect();

    let movement_event = resolve_tick(state, player, map, &enemy_positions, &npc_positions);
    let tile_event = if let Some((dx, dy)) = movement_event.step {
        if let Some(next_x) = player.x.checked_add_signed(dx as isize) {
            if let Some(next_y) = player.y.checked_add_signed(dy as isize) {
                tile_event_for_position(&player.current_map_id, next_x, next_y, data)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let map_changed = tile_event.as_ref().is_some_and(|event| match event {
        TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
            !target.is_empty() && data.find_map(target).is_some()
        }
        TileEvent::Treasure => false,
    });

    MovementUpdateResult {
        movement_event,
        tile_event,
        map_changed,
    }
}

pub fn resolve_tick(
    state: &MovementState,
    player: &CharacterState,
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

fn can_move(player: &CharacterState, map: &Map, dx: i32, dy: i32) -> bool {
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

fn tile_event_for_position(map_id: &str, x: usize, y: usize, data: &GameData) -> Option<TileEvent> {
    let map = data.find_map(map_id)?;
    let tile = map.get_tile(x, y);

    match tile {
        Tile::Treasure => Some(TileEvent::Treasure),
        Tile::Exit => {
            for (ex, ey, target) in &map.exits {
                if *ex == x && *ey == y {
                    return Some(TileEvent::MapExit(target.clone()));
                }
            }
            None
        }
        Tile::Dungeon => {
            for (dx, dy, target) in &map.dungeons {
                if *dx == x && *dy == y {
                    return Some(TileEvent::DungeonEntrance(target.clone()));
                }
            }
            None
        }
        _ => None,
    }
}

struct UpdateMovementResolver;

static UPDATE_MOVEMENT_RESOLVER: UpdateMovementResolver = UpdateMovementResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&UPDATE_MOVEMENT_RESOLVER]
}

impl DomainEventResolver for UpdateMovementResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::UpdateMovement)
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, _event: &GameEvent) -> Result<Vec<GameEvent>> {
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        let movement = resolve_world_tick(&s.movement, &s.leader, &s.combat.enemies, ctx.data());

        let mut events = Vec::with_capacity(if movement.map_changed { 2 } else { 1 });
        events.push(GameEvent::Movement(MovementEvent::Tick(
            movement.movement_event,
            movement.tile_event,
        )));
        if movement.map_changed {
            events.push(GameEvent::Transition(TransitionEvent::MapChanged));
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Direction, Map, Tile};
    use crate::game::{CharacterState, GameData};

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

    fn make_player_at(x: usize, y: usize, map_id: &str) -> CharacterState {
        let mut player = CharacterState::new(String::from("Tester"), map_id);
        player.x = x;
        player.y = y;
        player
    }

    fn make_world_data() -> GameData {
        let mut data = GameData::default();
        data.maps.push(Map {
            id: String::from("field"),
            name: String::from("Field"),
            width: 3,
            height: 3,
            tiles: vec![
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
            encounters: Vec::new(),
            exits: vec![(2, 1, String::from("town"))],
            dungeons: vec![(1, 2, String::from("cave"))],
            npcs: Vec::new(),
            peaceful: false,
        });
        data.maps.push(Map {
            id: String::from("town"),
            name: String::from("Town"),
            width: 2,
            height: 2,
            tiles: vec![Tile::PlayerStart, Tile::Floor, Tile::Floor, Tile::Floor],
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: true,
        });
        data.maps.push(Map {
            id: String::from("cave"),
            name: String::from("Cave"),
            width: 2,
            height: 2,
            tiles: vec![Tile::PlayerStart, Tile::Floor, Tile::Floor, Tile::Floor],
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: false,
        });
        data
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

        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved = state.apply_tick(&mut player, event);

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

        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved = state.apply_tick(&mut player, event);

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

        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved_while_cooling = state.apply_tick(&mut player, event);
        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved_after_cooling = state.apply_tick(&mut player, event);

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

        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved = state.apply_tick(&mut player, event);

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

        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved = state.apply_tick(&mut player, event);

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

        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved = state.apply_tick(&mut player, event);

        assert!(!moved);
        assert!(player.x == 1 && player.y == 1);
        assert!(matches!(player.facing, Direction::Right));
    }

    #[test]
    fn resolve_world_tick_sets_treasure_tile_event_without_map_change() {
        let data = make_world_data();
        let enemies: Vec<FieldEnemy> = Vec::new();
        let state = MovementState {
            pressed_direction: Some(Direction::Down),
            move_cooldown: 0,
        };
        let player = make_player_at(1, 0, "field");

        let result = resolve_world_tick(&state, &player, &enemies, &data);

        assert!(matches!(result.tile_event, Some(TileEvent::Treasure)));
        assert!(!result.map_changed);
    }

    #[test]
    fn resolve_world_tick_sets_exit_tile_event_with_map_change() {
        let data = make_world_data();
        let enemies: Vec<FieldEnemy> = Vec::new();
        let state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };
        let player = make_player_at(1, 1, "field");

        let result = resolve_world_tick(&state, &player, &enemies, &data);

        assert!(matches!(result.tile_event, Some(TileEvent::MapExit(target)) if target == "town"));
        assert!(result.map_changed);
    }

    #[test]
    fn resolve_world_tick_sets_dungeon_tile_event_with_map_change() {
        let data = make_world_data();
        let enemies: Vec<FieldEnemy> = Vec::new();
        let state = MovementState {
            pressed_direction: Some(Direction::Down),
            move_cooldown: 0,
        };
        let player = make_player_at(1, 1, "field");

        let result = resolve_world_tick(&state, &player, &enemies, &data);

        assert!(
            matches!(result.tile_event, Some(TileEvent::DungeonEntrance(target)) if target == "cave")
        );
        assert!(result.map_changed);
    }

    #[test]
    fn resolve_world_tick_sets_no_tile_event_on_floor() {
        let data = make_world_data();
        let enemies: Vec<FieldEnemy> = Vec::new();
        let state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };
        let player = make_player_at(0, 0, "field");

        let result = resolve_world_tick(&state, &player, &enemies, &data);

        assert!(result.tile_event.is_none());
        assert!(!result.map_changed);
    }
}

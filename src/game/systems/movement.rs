use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::{Direction, Map, Tile};

use crate::game::systems::resolver::{DomainEventResolver, ResolveContext};
use crate::game::{
    CharacterState, GameData, GameEvent, GameEventKind, GameState, MovementEvent, MovementState,
    MovementTickEvent, TileEvent, WorldState,
};

const MOVE_COOLDOWN: u32 = 2;

pub struct MovementUpdateResult {
    pub movement_event: MovementTickEvent,
    pub tile_event: Option<TileEvent>,
}

pub fn resolve_world_tick(
    state: &MovementState,
    player: &CharacterState,
    session: &WorldState,
    data: &GameData,
) -> MovementUpdateResult {
    let map = data.find_map(&player.current_map_id);
    let movement_event = resolve_tick_with_occupancy(state, player, session, map);
    let tile_event = if let Some((dx, dy)) = movement_event.step {
        let next_x = (player.x as i32 + dx) as usize;
        let next_y = (player.y as i32 + dy) as usize;
        tile_event_for_position(&player.current_map_id, next_x, next_y, data)
    } else {
        None
    };

    MovementUpdateResult {
        movement_event,
        tile_event,
    }
}

fn resolve_tick_with_occupancy(
    state: &MovementState,
    player: &CharacterState,
    session: &WorldState,
    map: Option<&Map>,
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
    let new_x = (player.x as i32 + dx) as usize;
    let new_y = (player.y as i32 + dy) as usize;

    if can_move(player, map, dx, dy) && !is_occupied(map, new_x, new_y, session) {
        step = Some((dx, dy));
    }

    MovementTickEvent {
        next_state,
        facing: Some((dx, dy)),
        step,
    }
}

fn is_occupied(map: &Map, x: usize, y: usize, session: &WorldState) -> bool {
    if x >= map.width || y >= map.height {
        return true;
    }
    session.is_occupied(x, y)
}

fn can_move(player: &CharacterState, map: &Map, dx: i32, dy: i32) -> bool {
    let new_x = (player.x as i32 + dx) as usize;
    let new_y = (player.y as i32 + dy) as usize;
    map.get_tile(new_x, new_y).is_passable()
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
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::UpdateMovement]
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        _event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx.world.ok_or_else(|| anyhow!("No active world"))?;

        let movement = resolve_world_tick(&s.movement, &s.leader, s, ctx.data());

        let has_meaningful_movement = movement.movement_event.next_state != s.movement
            || movement.movement_event.facing.is_some()
            || movement.movement_event.step.is_some()
            || movement.tile_event.is_some();
        if has_meaningful_movement {
            out.push(GameEvent::Movement(MovementEvent::Tick(
                movement.movement_event,
                movement.tile_event,
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Direction, Enemy, Map, Tile};
    use crate::game::state::FieldEnemy;
    use crate::game::{CharacterState, CombatEvent, GameData, GameEvent, WorldEvent, WorldState};

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

    fn make_session_with_map(data: &GameData, map_id: &str) -> WorldState {
        let mut session = WorldState::empty();
        assert!(
            session
                .apply_event(
                    data,
                    &GameEvent::World(WorldEvent::SetPlayerMap(map_id.into()))
                )
                .is_ok()
        );
        session
    }

    fn resolve_tick(
        state: &MovementState,
        player: &CharacterState,
        map: Option<&Map>,
        enemy_positions: &[(usize, usize)],
        npc_positions: &[(usize, usize)],
    ) -> MovementTickEvent {
        let mut session = WorldState::empty();
        let Some(map_ref) = map else {
            return super::resolve_tick_with_occupancy(state, player, &session, map);
        };

        let mut data = GameData::default();
        let mut map_with_npcs = map_ref.clone();
        map_with_npcs.npcs = npc_positions
            .iter()
            .map(|(x, y)| (*x, *y, String::from("npc")))
            .collect();
        data.maps.push(map_with_npcs);

        assert!(
            session
                .apply_event(
                    &data,
                    &GameEvent::World(WorldEvent::SetPlayerMap(map_ref.id.clone())),
                )
                .is_ok()
        );

        let enemies: Vec<FieldEnemy> = enemy_positions
            .iter()
            .enumerate()
            .map(|(idx, (x, y))| {
                FieldEnemy::new(
                    Enemy {
                        id: format!("e{idx}"),
                        name: String::from("Enemy"),
                        hp: 10,
                        atk: 1,
                        def: 0,
                        exp: 0,
                        gold: 0,
                    },
                    *x,
                    *y,
                    (idx as u32) + 1,
                )
            })
            .collect();
        assert!(
            session
                .apply_event(
                    &data,
                    &GameEvent::Combat(CombatEvent::SetMapEnemies {
                        enemies,
                        respawn_positions: Vec::new(),
                        next_enemy_instance_id: 1,
                    }),
                )
                .is_ok()
        );

        super::resolve_tick_with_occupancy(state, player, &session, Some(map_ref))
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
        let player = make_player_at(1, 1, "test_map");
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
        let moved = state.apply_tick(event);

        assert!(!moved);
        assert!(state.move_cooldown == 1);
        assert!(event.step.is_none());
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
        let player = make_player_at(1, 1, "test_map");
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
        let moved = state.apply_tick(event);

        assert!(!moved);
        assert!(state.move_cooldown == 0);
        assert!(event.step.is_none());
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
        let player = make_player_at(1, 1, "test_map");
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
        let moved_while_cooling = state.apply_tick(event);
        let event = resolve_tick(
            &state,
            &player,
            Some(&map),
            &enemy_positions,
            &npc_positions,
        );
        let moved_after_cooling = state.apply_tick(event);

        assert!(!moved_while_cooling);
        assert!(moved_after_cooling);
        assert!(event.step == Some((1, 0)));
        assert!(event.facing == Some((1, 0)));
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
        let player = make_player_at(1, 1, "test_map");
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
        let moved = state.apply_tick(event);

        assert!(!moved);
        assert!(event.step.is_none());
        assert!(event.facing == Some((1, 0)));
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
        let player = make_player_at(1, 1, "test_map");
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
        let moved = state.apply_tick(event);

        assert!(!moved);
        assert!(event.step.is_none());
        assert!(event.facing == Some((1, 0)));
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
        let player = make_player_at(1, 1, "test_map");
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
        let moved = state.apply_tick(event);

        assert!(!moved);
        assert!(event.step.is_none());
        assert!(event.facing == Some((1, 0)));
    }

    #[test]
    fn resolve_world_tick_sets_treasure_tile_event_without_map_change() {
        let data = make_world_data();
        let session = make_session_with_map(&data, "field");
        let state = MovementState {
            pressed_direction: Some(Direction::Down),
            move_cooldown: 0,
        };
        let player = make_player_at(1, 0, "field");

        let result = resolve_world_tick(&state, &player, &session, &data);

        assert!(matches!(result.tile_event, Some(TileEvent::Treasure)));
    }

    #[test]
    fn resolve_world_tick_sets_exit_tile_event_with_map_change() {
        let data = make_world_data();
        let session = make_session_with_map(&data, "field");
        let state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };
        let player = make_player_at(1, 1, "field");

        let result = resolve_world_tick(&state, &player, &session, &data);

        assert!(matches!(result.tile_event, Some(TileEvent::MapExit(target)) if target == "town"));
    }

    #[test]
    fn resolve_world_tick_sets_dungeon_tile_event_with_map_change() {
        let data = make_world_data();
        let session = make_session_with_map(&data, "field");
        let state = MovementState {
            pressed_direction: Some(Direction::Down),
            move_cooldown: 0,
        };
        let player = make_player_at(1, 1, "field");

        let result = resolve_world_tick(&state, &player, &session, &data);

        assert!(
            matches!(result.tile_event, Some(TileEvent::DungeonEntrance(target)) if target == "cave")
        );
    }

    #[test]
    fn resolve_world_tick_sets_no_tile_event_on_floor() {
        let data = make_world_data();
        let session = make_session_with_map(&data, "field");
        let state = MovementState {
            pressed_direction: Some(Direction::Right),
            move_cooldown: 0,
        };
        let player = make_player_at(0, 0, "field");

        let result = resolve_world_tick(&state, &player, &session, &data);

        assert!(result.tile_event.is_none());
    }
}

use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{Direction, Map, Tile};

use crate::game::state::EntityState;
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{
    GameData, GameEvent, GameEventKind, MovementEvent, MovementState, MovementTickEvent, TileEvent,
    WorldState,
};

const MOVE_COOLDOWN: u32 = 2;

pub fn resolve_world_tick(
    state: &MovementState,
    leader: &EntityState,
    world: &WorldState,
    data: &GameData,
) -> (MovementTickEvent, Option<TileEvent>) {
    let map = data.find_map(&leader.map_id);
    let movement_event = resolve_tick_with_occupancy(state, leader, world, map);
    let tile_event = if let Some((dx, dy)) = movement_event.step {
        let next_x = (leader.x as i32 + dx).max(0) as usize;
        let next_y = (leader.y as i32 + dy).max(0) as usize;
        tile_event_for_position(&leader.map_id, next_x, next_y, data)
    } else {
        None
    };

    (movement_event, tile_event)
}

fn can_move(entity: &EntityState, map: &Map, dx: i32, dy: i32) -> bool {
    let raw_x = entity.x as i32 + dx;
    let raw_y = entity.y as i32 + dy;
    if raw_x < 0 || raw_y < 0 {
        return false;
    }
    map.get_tile(raw_x as usize, raw_y as usize).is_passable()
}

fn resolve_tick_with_occupancy(
    state: &MovementState,
    leader: &EntityState,
    world: &WorldState,
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
    let new_x = (leader.x as i32 + dx).max(0) as usize;
    let new_y = (leader.y as i32 + dy).max(0) as usize;

    if can_move(leader, map, dx, dy) && !world.is_occupied_on_map(map, new_x, new_y) {
        step = Some((dx, dy));
    }

    MovementTickEvent {
        next_state,
        facing: Some((dx, dy)),
        step,
    }
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
        data: &Rc<GameData>,
        world: Option<&WorldState>,
        _event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let world = world.ok_or_else(|| anyhow!("No active world"))?;
        let leader = world
            .leader_entity()
            .ok_or_else(|| anyhow!("No leader entity"))?;

        let (movement_event, tile_event) = resolve_world_tick(&world.movement, leader, world, data);

        let has_meaningful_movement = movement_event.next_state != world.movement
            || movement_event.facing.is_some()
            || movement_event.step.is_some()
            || tile_event.is_some();
        if has_meaningful_movement {
            out.push(GameEvent::Movement(MovementEvent::Tick(
                movement_event,
                tile_event,
            )));
        }
        Ok(())
    }
}

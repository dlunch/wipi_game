use wipi::event::KeyCode;

use super::combat::{CombatState, enemy_at};
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
) {
    if !matches!(game_state, GameState::Explore) {
        return;
    }

    let map_id = player.current_map_id.clone();
    let Some(map) = data.find_map(&map_id) else {
        return;
    };

    let moved = tick(state, player, map, combat, &data.npcs);

    if moved {
        game::explore::check_tile_events(player, combat, data);
    }
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

pub fn tick(
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

    if !player.can_move(map, dx, dy) {
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

use wipi::event::KeyCode;

use super::combat::{enemy_at, CombatState};
use crate::data::{Map, Npc};
use crate::game::Player;

const MOVE_COOLDOWN: u32 = 5;

pub enum MovementIntent {
    DirectionPressed(KeyCode),
    KeyReleased(KeyCode),
    Tick,
}

pub struct MovementContext<'a> {
    pub player: &'a mut Player,
    pub map: &'a Map,
    pub combat: &'a CombatState,
    pub npcs: &'a [Npc],
}

#[derive(Default)]
pub struct MovementState {
    pub pressed_direction: Option<KeyCode>,
    pub move_cooldown: u32,
}

pub fn reduce(
    state: &mut MovementState,
    intent: MovementIntent,
    ctx: Option<MovementContext<'_>>,
) -> bool {
    match intent {
        MovementIntent::DirectionPressed(key) => {
            on_direction_pressed(state, key);
            false
        }
        MovementIntent::KeyReleased(key) => {
            on_key_released(state, key);
            false
        }
        MovementIntent::Tick => {
            let Some(ctx) = ctx else {
                return false;
            };
            update(state, ctx.player, ctx.map, ctx.combat, ctx.npcs)
        }
    }
}

fn on_direction_pressed(state: &mut MovementState, key: KeyCode) {
    state.pressed_direction = Some(key);
    state.move_cooldown = 0;
}

fn on_key_released(state: &mut MovementState, key: KeyCode) {
    if state.pressed_direction == Some(key) {
        state.pressed_direction = None;
    }
}

fn update(
    state: &mut MovementState,
    player: &mut Player,
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
    player: &mut Player,
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

    player.set_facing(dx, dy);

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

    player.move_by(dx, dy);
    true
}

fn npc_at(npcs: &[Npc], map_id: &str, x: usize, y: usize) -> bool {
    npcs.iter()
        .any(|npc| npc.map_id == map_id && npc.x == x && npc.y == y)
}

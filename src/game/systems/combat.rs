use alloc::vec::Vec;

use crate::data::{Enemy, Map};
use crate::game::state::{CombatState, FieldEnemy};

const ENEMY_MOVE_INTERVAL: u32 = 8;
const MP_REGEN_INTERVAL: u32 = 60;

pub struct CombatTickInput<'a> {
    pub player_x: usize,
    pub player_y: usize,
    pub player_def: i32,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
    pub map: &'a Map,
    pub enemy_data: &'a [Enemy],
}

struct TickContext<'a> {
    player_x: usize,
    player_y: usize,
    player_def: i32,
    skill_cooldowns: [u32; 3],
    mp_regen_timer: u32,
    map: &'a Map,
    enemy_data: &'a [Enemy],
}

#[derive(Debug, Clone)]
pub struct CombatResult {
    pub damage_taken: i32,
    pub next_skill_cooldowns: [u32; 3],
    pub next_mp_regen_timer: u32,
    pub recover_mp: i32,
    pub next_state: CombatState,
}

pub fn reduce_tick(state: &CombatState, input: CombatTickInput<'_>) -> CombatResult {
    let mut next_state = state.clone();
    let mut result = update(
        &mut next_state,
        TickContext {
            player_x: input.player_x,
            player_y: input.player_y,
            player_def: input.player_def,
            skill_cooldowns: input.skill_cooldowns,
            mp_regen_timer: input.mp_regen_timer,
            map: input.map,
            enemy_data: input.enemy_data,
        },
    );
    result.next_state = next_state;
    result
}

fn update(state: &mut CombatState, ctx: TickContext<'_>) -> CombatResult {
    state.update_counter = state.update_counter.wrapping_add(1);

    if state.player_attack_cooldown > 0 {
        state.player_attack_cooldown -= 1;
    }
    if state.player_hit_flash > 0 {
        state.player_hit_flash -= 1;
    }

    for effect in &mut state.skill_effects {
        if effect.timer > 0 {
            effect.timer -= 1;
        }
    }
    state.skill_effects.retain(|e| e.timer > 0);

    let mut damage_taken = 0;

    if state.update_counter.is_multiple_of(ENEMY_MOVE_INTERVAL) {
        for enemy in &mut state.enemies {
            if !enemy.is_dead() {
                enemy.update(ctx.player_x, ctx.player_y, ctx.map);
            }
        }
    }

    for enemy in &mut state.enemies {
        if enemy.is_dead() {
            continue;
        }

        if enemy.distance_to(ctx.player_x, ctx.player_y) <= 1 && enemy.can_attack() {
            let raw_damage = enemy.do_attack();
            let actual_damage = (raw_damage - ctx.player_def / 2).max(1);
            damage_taken += actual_damage;
            state.player_hit_flash = 10;
        }
    }

    state.enemies.retain(|e| !e.is_dead());

    try_respawn(state, ctx.player_x, ctx.player_y, ctx.map, ctx.enemy_data);

    let (next_skill_cooldowns, next_mp_regen_timer, recover_mp) =
        tick_resource_state(ctx.skill_cooldowns, ctx.mp_regen_timer);

    CombatResult {
        damage_taken,
        next_skill_cooldowns,
        next_mp_regen_timer,
        recover_mp,
        next_state: state.clone(),
    }
}

fn try_respawn(
    state: &mut CombatState,
    player_x: usize,
    player_y: usize,
    map: &Map,
    enemy_data: &[Enemy],
) {
    const RESPAWN_DELAY: u32 = 300;
    const RESPAWN_DISTANCE: usize = 8;

    if state.respawn_positions.is_empty() {
        return;
    }

    let max_enemies = state.respawn_positions.len();
    if state.enemies.len() >= max_enemies {
        state.respawn_timer = 0;
        return;
    }

    state.respawn_timer += 1;
    if state.respawn_timer < RESPAWN_DELAY {
        return;
    }

    let available_enemies: Vec<&Enemy> = map
        .encounters
        .iter()
        .filter_map(|(id, _)| enemy_data.iter().find(|e| &e.id == id))
        .collect();

    if available_enemies.is_empty() {
        return;
    }

    for (x, y, enemy_idx) in &state.respawn_positions {
        let distance = x.abs_diff(player_x) + y.abs_diff(player_y);
        if distance < RESPAWN_DISTANCE {
            continue;
        }

        let already_exists = state.enemies.iter().any(|e| e.x == *x && e.y == *y);
        if already_exists {
            continue;
        }

        if let Some(enemy) = available_enemies.get(*enemy_idx) {
            state
                .enemies
                .push(FieldEnemy::new((*enemy).clone(), *x, *y));
            state.respawn_timer = 0;
            return;
        }
    }
}

fn tick_resource_state(skill_cooldowns: [u32; 3], mp_regen_timer: u32) -> ([u32; 3], u32, i32) {
    let mut next_skill_cooldowns = skill_cooldowns;
    for cooldown in &mut next_skill_cooldowns {
        if *cooldown > 0 {
            *cooldown -= 1;
        }
    }

    let mut next_mp_regen_timer = mp_regen_timer + 1;
    let mut recover_mp = 0;
    if next_mp_regen_timer >= MP_REGEN_INTERVAL {
        next_mp_regen_timer = 0;
        recover_mp = 1;
    }

    (next_skill_cooldowns, next_mp_regen_timer, recover_mp)
}

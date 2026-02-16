use alloc::vec::Vec;

use crate::data::{Enemy, Map};
use crate::game::state::{CombatState, FieldEnemy};
use crate::game::{CombatRuntimeEvent, RuntimeEvent};

const ENEMY_MOVE_INTERVAL: u32 = 8;
const MP_REGEN_INTERVAL: u32 = 60;

pub fn resolve_tick(
    state: &CombatState,
    player_x: usize,
    player_y: usize,
    player_def: i32,
    resources: ([u32; 3], u32),
    map: &Map,
    enemy_data: &[Enemy],
) -> Vec<RuntimeEvent> {
    let (skill_cooldowns, mp_regen_timer) = resources;
    let update_counter = state.update_counter.wrapping_add(1);
    let mut player_attack_cooldown = state.player_attack_cooldown;
    let mut player_hit_flash = state.player_hit_flash;
    let mut skill_effects = state.skill_effects.clone();
    let mut enemies = state.enemies.clone();
    let mut respawn_timer = state.respawn_timer;

    player_attack_cooldown = player_attack_cooldown.saturating_sub(1);
    player_hit_flash = player_hit_flash.saturating_sub(1);

    for effect in &mut skill_effects {
        if effect.timer > 0 {
            effect.timer -= 1;
        }
    }
    skill_effects.retain(|e| e.timer > 0);

    let mut damage_taken = 0;

    if update_counter.is_multiple_of(ENEMY_MOVE_INTERVAL) {
        for enemy in &mut enemies {
            if !enemy.is_dead() {
                enemy.update(player_x, player_y, map);
            }
        }
    }

    for enemy in &mut enemies {
        if enemy.is_dead() {
            continue;
        }

        if enemy.distance_to(player_x, player_y) <= 1 && enemy.can_attack() {
            let raw_damage = enemy.do_attack();
            let actual_damage = (raw_damage - player_def / 2).max(1);
            damage_taken += actual_damage;
            player_hit_flash = 10;
        }
    }

    enemies.retain(|e| !e.is_dead());

    try_respawn(
        &mut enemies,
        &mut respawn_timer,
        &state.respawn_positions,
        player_x,
        player_y,
        map,
        enemy_data,
    );

    let (next_skill_cooldowns, next_mp_regen_timer, recover_mp) =
        tick_resource_state(skill_cooldowns, mp_regen_timer);
    let mut events = Vec::with_capacity(10);
    events.push(RuntimeEvent::Combat(CombatRuntimeEvent::SetEnemies(
        enemies,
    )));
    events.push(RuntimeEvent::Combat(
        CombatRuntimeEvent::SetPlayerAttackCooldown(player_attack_cooldown),
    ));
    events.push(RuntimeEvent::Combat(CombatRuntimeEvent::SetPlayerHitFlash(
        player_hit_flash,
    )));
    events.push(RuntimeEvent::Combat(CombatRuntimeEvent::SetSkillEffects(
        skill_effects,
    )));
    events.push(RuntimeEvent::Combat(CombatRuntimeEvent::SetUpdateCounter(
        update_counter,
    )));
    events.push(RuntimeEvent::Combat(CombatRuntimeEvent::SetRespawnTimer(
        respawn_timer,
    )));
    events.push(RuntimeEvent::Combat(CombatRuntimeEvent::SetSkillCooldowns(
        next_skill_cooldowns,
    )));
    events.push(RuntimeEvent::Combat(CombatRuntimeEvent::SetMpRegenTimer(
        next_mp_regen_timer,
    )));
    if recover_mp > 0 {
        events.push(RuntimeEvent::Combat(CombatRuntimeEvent::RecoverMp(
            recover_mp,
        )));
    }
    if damage_taken > 0 {
        events.push(RuntimeEvent::Combat(CombatRuntimeEvent::TakeDamage(
            damage_taken,
        )));
    }

    events
}

fn try_respawn(
    enemies: &mut Vec<FieldEnemy>,
    respawn_timer: &mut u32,
    respawn_positions: &[(usize, usize, usize)],
    player_x: usize,
    player_y: usize,
    map: &Map,
    enemy_data: &[Enemy],
) {
    const RESPAWN_DELAY: u32 = 300;
    const RESPAWN_DISTANCE: usize = 8;

    if respawn_positions.is_empty() {
        return;
    }

    let max_enemies = respawn_positions.len();
    if enemies.len() >= max_enemies {
        *respawn_timer = 0;
        return;
    }

    *respawn_timer += 1;
    if *respawn_timer < RESPAWN_DELAY {
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

    for (x, y, enemy_idx) in respawn_positions {
        let distance = x.abs_diff(player_x) + y.abs_diff(player_y);
        if distance < RESPAWN_DISTANCE {
            continue;
        }

        let already_exists = enemies.iter().any(|e| e.x == *x && e.y == *y);
        if already_exists {
            continue;
        }

        if let Some(enemy) = available_enemies.get(*enemy_idx) {
            enemies.push(FieldEnemy::new((*enemy).clone(), *x, *y));
            *respawn_timer = 0;
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

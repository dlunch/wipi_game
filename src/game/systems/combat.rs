use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::{Enemy, Map};

use crate::game::state::{CombatState, FieldEnemy};
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{CombatRuntimeEvent, GameEvent, GameState};

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
) -> Vec<GameEvent> {
    let (skill_cooldowns, mp_regen_timer) = resources;
    let mut events = Vec::with_capacity(12);
    let update_counter = state.update_counter.wrapping_add(1);
    events.push(GameEvent::Combat(CombatRuntimeEvent::SetUpdateCounter(
        update_counter,
    )));

    let mut player_attack_cooldown = state.player_attack_cooldown;
    let mut player_hit_flash = state.player_hit_flash;
    let mut skill_effects = state.skill_effects.clone();
    let mut enemies = state.enemies.clone();
    let mut respawn_timer = state.respawn_timer;
    let mut next_enemy_instance_id = state.next_enemy_instance_id;

    if player_attack_cooldown > 0 {
        player_attack_cooldown = player_attack_cooldown.saturating_sub(1);
        events.push(GameEvent::Combat(
            CombatRuntimeEvent::SetPlayerAttackCooldown(player_attack_cooldown),
        ));
    }
    if player_hit_flash > 0 {
        player_hit_flash = player_hit_flash.saturating_sub(1);
        events.push(GameEvent::Combat(CombatRuntimeEvent::SetPlayerHitFlash(
            player_hit_flash,
        )));
    }

    if !skill_effects.is_empty() {
        for effect in &mut skill_effects {
            if effect.timer > 0 {
                effect.timer -= 1;
            }
        }
        skill_effects.retain(|e| e.timer > 0);
        events.push(GameEvent::Combat(CombatRuntimeEvent::SetSkillEffects(
            skill_effects.clone(),
        )));
    }

    let mut damage_taken = 0;

    if update_counter.is_multiple_of(ENEMY_MOVE_INTERVAL) {
        for enemy in &mut enemies {
            if !enemy.is_dead() {
                enemy.update(player_x, player_y, map);
            }
        }
    }

    let previous_enemies = state.enemies.clone();
    for enemy in &mut enemies {
        if enemy.is_dead() {
            continue;
        }

        if enemy.distance_to(player_x, player_y) <= 1 && enemy.can_attack() {
            let raw_damage = enemy.do_attack();
            let actual_damage = (raw_damage - player_def / 2).max(1);
            damage_taken += actual_damage;
            if player_hit_flash != 10 {
                player_hit_flash = 10;
                events.push(GameEvent::Combat(CombatRuntimeEvent::SetPlayerHitFlash(
                    player_hit_flash,
                )));
            }
        }
    }

    enemies.retain(|e| !e.is_dead());

    try_respawn(
        &mut enemies,
        &mut respawn_timer,
        &mut next_enemy_instance_id,
        &state.respawn_positions,
        (player_x, player_y),
        map,
        enemy_data,
    );

    push_enemy_events(&previous_enemies, &enemies, &mut events);
    if respawn_timer != state.respawn_timer {
        events.push(GameEvent::Combat(CombatRuntimeEvent::SetRespawnTimer(
            respawn_timer,
        )));
    }
    if next_enemy_instance_id != state.next_enemy_instance_id {
        events.push(GameEvent::Combat(
            CombatRuntimeEvent::SetNextEnemyInstanceId(next_enemy_instance_id),
        ));
    }

    tick_resource_state(skill_cooldowns, mp_regen_timer, &mut events);
    if damage_taken > 0 {
        events.push(GameEvent::Combat(CombatRuntimeEvent::TakeDamage(
            damage_taken,
        )));
    }

    events
}

fn try_respawn(
    enemies: &mut Vec<FieldEnemy>,
    respawn_timer: &mut u32,
    next_enemy_instance_id: &mut u32,
    respawn_positions: &[(usize, usize, usize)],
    player_pos: (usize, usize),
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
        let distance = x.abs_diff(player_pos.0) + y.abs_diff(player_pos.1);
        if distance < RESPAWN_DISTANCE {
            continue;
        }

        let already_exists = enemies.iter().any(|e| e.x == *x && e.y == *y);
        if already_exists {
            continue;
        }

        if let Some(enemy) = available_enemies.get(*enemy_idx) {
            let instance_id = allocate_enemy_instance_id(next_enemy_instance_id);
            enemies.push(FieldEnemy::new((*enemy).clone(), *x, *y, instance_id));
            *respawn_timer = 0;
            return;
        }
    }
}

fn tick_resource_state(
    skill_cooldowns: [u32; 3],
    mp_regen_timer: u32,
    events: &mut Vec<GameEvent>,
) {
    let mut next_skill_cooldowns = skill_cooldowns;
    for cooldown in &mut next_skill_cooldowns {
        if *cooldown > 0 {
            *cooldown -= 1;
        }
    }
    if next_skill_cooldowns != skill_cooldowns {
        events.push(GameEvent::Combat(CombatRuntimeEvent::SetSkillCooldowns(
            next_skill_cooldowns,
        )));
    }

    let mut next_mp_regen_timer = mp_regen_timer + 1;
    let mut recover_mp = false;
    if next_mp_regen_timer >= MP_REGEN_INTERVAL {
        next_mp_regen_timer = 0;
        recover_mp = true;
    }
    if next_mp_regen_timer != mp_regen_timer {
        events.push(GameEvent::Combat(CombatRuntimeEvent::SetMpRegenTimer(
            next_mp_regen_timer,
        )));
    }
    if recover_mp {
        events.push(GameEvent::Combat(CombatRuntimeEvent::RecoverMp(1)));
    }
}

fn push_enemy_events(previous: &[FieldEnemy], next: &[FieldEnemy], events: &mut Vec<GameEvent>) {
    for enemy in previous {
        if !next
            .iter()
            .any(|next_enemy| next_enemy.instance_id == enemy.instance_id)
        {
            events.push(GameEvent::Combat(CombatRuntimeEvent::EnemyDespawn(
                enemy.instance_id,
            )));
        }
    }

    for enemy in next {
        let Some(previous_enemy) = previous
            .iter()
            .find(|previous_enemy| previous_enemy.instance_id == enemy.instance_id)
        else {
            events.push(GameEvent::Combat(CombatRuntimeEvent::EnemySpawn(
                enemy.clone(),
            )));
            continue;
        };

        if previous_enemy.x != enemy.x || previous_enemy.y != enemy.y {
            events.push(GameEvent::Combat(CombatRuntimeEvent::EnemyMove {
                enemy_id: enemy.instance_id,
                x: enemy.x,
                y: enemy.y,
            }));
        }
        if previous_enemy.hp != enemy.hp {
            events.push(GameEvent::Combat(CombatRuntimeEvent::EnemyHpSet {
                enemy_id: enemy.instance_id,
                hp: enemy.hp,
            }));
        }
        if previous_enemy.attack_cooldown != enemy.attack_cooldown {
            events.push(GameEvent::Combat(
                CombatRuntimeEvent::EnemyAttackCooldownSet {
                    enemy_id: enemy.instance_id,
                    cooldown: enemy.attack_cooldown,
                },
            ));
        }
        if previous_enemy.hit_flash != enemy.hit_flash {
            events.push(GameEvent::Combat(CombatRuntimeEvent::EnemyHitFlashSet {
                enemy_id: enemy.instance_id,
                hit_flash: enemy.hit_flash,
            }));
        }
    }
}

struct UpdateCombatResolver;

static UPDATE_COMBAT_RESOLVER: UpdateCombatResolver = UpdateCombatResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&UPDATE_COMBAT_RESOLVER]
}

impl DomainEventResolver for UpdateCombatResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::UpdateCombat)
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, _event: &GameEvent) -> Result<Vec<GameEvent>> {
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;
        let Some(map) = ctx.data().find_map(&s.player.current_map_id) else {
            return Ok(Vec::new());
        };

        Ok(resolve_tick(
            &s.combat,
            s.player.x,
            s.player.y,
            s.player.total_def(),
            (s.skill_cooldowns, s.mp_regen_timer),
            map,
            &ctx.data().enemies,
        ))
    }
}

fn allocate_enemy_instance_id(next_enemy_instance_id: &mut u32) -> u32 {
    let id = (*next_enemy_instance_id).max(1);
    *next_enemy_instance_id = next_enemy_instance_id.wrapping_add(1);
    if *next_enemy_instance_id == 0 {
        *next_enemy_instance_id = 1;
    }
    id
}

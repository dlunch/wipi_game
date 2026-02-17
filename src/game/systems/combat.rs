use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::{Direction, Enemy, Map, Skill, SkillType};

use crate::game::state::{CombatState, FieldEnemy, KillReward, SkillEffect};
use crate::game::systems::resolver::{DomainEventResolver, ResolveContext};
use crate::game::{CombatEvent, GameEvent, GameEventKind, GameState, TransitionEvent, WorldEvent};

const ENEMY_MOVE_INTERVAL: u32 = 8;
const MP_REGEN_INTERVAL: u32 = 60;
const HIT_FLASH_DURATION: u32 = 10;
const ENEMY_ATTACK_COOLDOWN: u32 = 30;
const PLAYER_ATTACK_COOLDOWN: u32 = 15;
const ATTACK_EFFECT_DURATION: u32 = 6;
const SKILL_EFFECT_DURATION: u32 = 8;
const HEAL_EFFECT_DURATION: u32 = 15;

pub fn resolve_tick(
    state: &CombatState,
    player_pos: (usize, usize),
    player_def: i32,
    resources: ([u32; 3], u32),
    map: &Map,
    enemy_data: &[Enemy],
    out: &mut Vec<GameEvent>,
) {
    // UpdateCombat tick can emit multiple enemy-related events in one frame.
    // Reserve upfront to avoid repeated growth reallocations on embedded targets.
    out.reserve(state.enemies.len() * 4 + 16);

    let (player_x, player_y) = player_pos;
    let (skill_cooldowns, mp_regen_timer) = resources;
    let update_counter = state.update_counter.wrapping_add(1);

    let player_attack_cooldown = if state.player_attack_cooldown > 0 {
        state.player_attack_cooldown - 1
    } else {
        0
    };
    if player_attack_cooldown != state.player_attack_cooldown {
        out.push(GameEvent::Combat(CombatEvent::SetPlayerAttackCooldown(
            player_attack_cooldown,
        )));
    }

    if !state.skill_effects.is_empty() {
        out.push(GameEvent::Combat(CombatEvent::TickSkillEffects));
    }

    let mut damage_taken = 0;
    let mut player_hit_flash_started = false;
    let mut occupied_after_tick: Vec<(usize, usize)> = Vec::with_capacity(state.enemies.len());
    let do_move = update_counter.is_multiple_of(ENEMY_MOVE_INTERVAL);
    for enemy in &state.enemies {
        if enemy.hp <= 0 {
            out.push(GameEvent::Combat(CombatEvent::EnemyDespawn(
                enemy.instance_id,
            )));
            continue;
        }

        let mut next_x = enemy.x;
        let mut next_y = enemy.y;
        let mut next_attack_cooldown = if enemy.attack_cooldown > 0 {
            enemy.attack_cooldown - 1
        } else {
            0
        };

        if do_move && enemy_distance_to(enemy, player_x, player_y) > 1 {
            (next_x, next_y) = next_enemy_position(enemy, player_x, player_y, map);
        }
        if enemy_distance(next_x, next_y, player_x, player_y) <= 1 && next_attack_cooldown == 0 {
            let raw_damage = enemy.data.atk;
            next_attack_cooldown = ENEMY_ATTACK_COOLDOWN;
            let actual_damage = (raw_damage - player_def / 2).max(1);
            damage_taken += actual_damage;
            if !player_hit_flash_started {
                player_hit_flash_started = true;
                out.push(GameEvent::Combat(CombatEvent::SetPlayerHitFlash(
                    HIT_FLASH_DURATION,
                )));
            }
        }

        if next_x != enemy.x || next_y != enemy.y {
            out.push(GameEvent::Combat(CombatEvent::EnemyMove {
                enemy_id: enemy.instance_id,
                x: next_x,
                y: next_y,
            }));
        }
        if next_attack_cooldown != enemy.attack_cooldown {
            out.push(GameEvent::Combat(CombatEvent::EnemyAttackCooldownSet {
                enemy_id: enemy.instance_id,
                cooldown: next_attack_cooldown,
            }));
        }
        occupied_after_tick.push((next_x, next_y));
    }

    resolve_respawn(
        state.respawn_timer,
        state.next_enemy_instance_id,
        state.respawn_positions.as_slice(),
        occupied_after_tick.as_slice(),
        (player_x, player_y),
        enemy_data,
        out,
    );

    tick_resource_state(skill_cooldowns, mp_regen_timer, out);
    out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(
        update_counter,
    )));
    if damage_taken > 0 {
        out.push(GameEvent::Combat(CombatEvent::TakeDamage(damage_taken)));
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
        events.push(GameEvent::Combat(CombatEvent::SetSkillCooldowns(
            next_skill_cooldowns,
        )));
    }

    let mut next_mp_regen_timer = mp_regen_timer + 1;
    if next_mp_regen_timer >= MP_REGEN_INTERVAL {
        next_mp_regen_timer = 0;
        events.push(GameEvent::Combat(CombatEvent::RecoverMp(1)));
    }
    if next_mp_regen_timer != mp_regen_timer {
        events.push(GameEvent::World(WorldEvent::SetMpRegenTimer(
            next_mp_regen_timer,
        )));
    }
}

struct CombatResolver;

static COMBAT_RESOLVER: CombatResolver = CombatResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&COMBAT_RESOLVER]
}

impl DomainEventResolver for CombatResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[
            GameEventKind::UpdateCombat,
            GameEventKind::CombatPlayerAction,
            GameEventKind::Transition,
        ]
    }

    fn resolve(
        &self,
        ctx: &ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::UpdateCombat => {
                ensure!(
                    matches!(ctx.state, GameState::Explore),
                    "Invalid state: expected Explore"
                );
                let data = ctx.data.as_ref();
                let s = ctx.world.ok_or_else(|| anyhow!("No active world"))?;
                let Some(map) = data.find_map(&s.leader.current_map_id) else {
                    return Ok(());
                };
                resolve_tick(
                    &s.combat,
                    (s.leader.x, s.leader.y),
                    s.leader.total_def(),
                    (s.skill_cooldowns, s.mp_regen_timer),
                    map,
                    &data.enemies,
                    out,
                );
            }
            GameEvent::CombatPlayerAction(action) => {
                ensure!(
                    matches!(ctx.state, GameState::Explore),
                    "Invalid state: expected Explore"
                );
                let s = ctx.world.ok_or_else(|| anyhow!("No active world"))?;

                if let Some((slot, skill)) = action.skill() {
                    if !s
                        .leader
                        .can_use_skill(&s.skill_cooldowns, slot, skill.mp_cost)
                    {
                        return Ok(());
                    }

                    out.push(GameEvent::Combat(CombatEvent::SetSkillCooldowns({
                        let mut next_skill_cooldowns = s.skill_cooldowns;
                        next_skill_cooldowns[slot] = skill.cooldown;
                        next_skill_cooldowns
                    })));
                    out.push(GameEvent::Combat(CombatEvent::RecoverMp(-skill.mp_cost)));
                    resolve_skill_action(
                        &s.combat,
                        skill,
                        s.leader.x,
                        s.leader.y,
                        s.leader.total_atk(),
                        s.leader.facing,
                        out,
                    );
                } else {
                    resolve_player_attack_action(
                        &s.combat,
                        s.leader.x,
                        s.leader.y,
                        s.leader.total_atk(),
                        s.leader.facing,
                        out,
                    );
                }
            }
            GameEvent::Transition(TransitionEvent::MapChanged) => {
                let data = ctx.data.as_ref();
                let session = ctx.world.ok_or_else(|| anyhow!("No active world"))?;
                let Some(map) = data.find_map(&session.leader.current_map_id) else {
                    return Ok(());
                };
                build_map_enemies(map, &data.enemies, out);
            }
            _ => {}
        }
        Ok(())
    }
}

fn allocate_enemy_instance_id(next_enemy_instance_id: u32) -> (u32, u32) {
    let id = next_enemy_instance_id.max(1);
    let mut next = next_enemy_instance_id.wrapping_add(1);
    if next == 0 {
        next = 1;
    }
    (id, next)
}

fn enemy_distance_to(enemy: &FieldEnemy, px: usize, py: usize) -> usize {
    enemy_distance(enemy.x, enemy.y, px, py)
}

fn enemy_distance(x: usize, y: usize, px: usize, py: usize) -> usize {
    x.abs_diff(px) + y.abs_diff(py)
}

fn next_enemy_position(
    enemy: &FieldEnemy,
    target_x: usize,
    target_y: usize,
    map: &Map,
) -> (usize, usize) {
    let dx: i32 = match target_x.cmp(&enemy.x) {
        core::cmp::Ordering::Greater => 1,
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
    };
    let dy: i32 = match target_y.cmp(&enemy.y) {
        core::cmp::Ordering::Greater => 1,
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
    };

    let new_x = if dx != 0 {
        Some((enemy.x as i32 + dx) as usize)
    } else {
        None
    };
    if let Some(nx) = new_x
        && dx != 0
        && map.get_tile(nx, enemy.y).is_passable()
    {
        return (nx, enemy.y);
    }
    let new_y = if dy != 0 {
        Some((enemy.y as i32 + dy) as usize)
    } else {
        None
    };
    if let Some(ny) = new_y
        && dy != 0
        && map.get_tile(enemy.x, ny).is_passable()
    {
        return (enemy.x, ny);
    }
    (enemy.x, enemy.y)
}

fn resolve_respawn(
    current_timer: u32,
    current_next_enemy_instance_id: u32,
    respawn_positions: &[(usize, usize, usize)],
    occupied_positions: &[(usize, usize)],
    player_pos: (usize, usize),
    enemy_data: &[Enemy],
    out: &mut Vec<GameEvent>,
) {
    const RESPAWN_DELAY: u32 = 300;
    const RESPAWN_DISTANCE: usize = 8;

    if respawn_positions.is_empty() {
        return;
    }

    if occupied_positions.len() >= respawn_positions.len() {
        if current_timer != 0 {
            out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
        }
        return;
    }

    let next_timer = current_timer.wrapping_add(1);
    if next_timer < RESPAWN_DELAY {
        if next_timer != current_timer {
            out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(next_timer)));
        }
        return;
    }

    for (x, y, enemy_data_idx) in respawn_positions {
        let distance = x.abs_diff(player_pos.0) + y.abs_diff(player_pos.1);
        if distance < RESPAWN_DISTANCE {
            continue;
        }
        if occupied_positions
            .iter()
            .any(|(occupied_x, occupied_y)| occupied_x == x && occupied_y == y)
        {
            continue;
        }
        if let Some(enemy) = enemy_data.get(*enemy_data_idx) {
            let (instance_id, next_id) = allocate_enemy_instance_id(current_next_enemy_instance_id);
            out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
            out.push(GameEvent::Combat(CombatEvent::EnemySpawn(FieldEnemy::new(
                enemy.clone(),
                *x,
                *y,
                instance_id,
            ))));
            if next_id != current_next_enemy_instance_id {
                out.push(GameEvent::Combat(CombatEvent::SetNextEnemyInstanceId(
                    next_id,
                )));
            }
            return;
        }
    }

    if next_timer != current_timer {
        out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(next_timer)));
    }
}

fn build_map_enemies(map: &Map, enemy_data: &[Enemy], out: &mut Vec<GameEvent>) {
    let mut enemies = Vec::with_capacity(map.encounters.len().max(4));
    let mut respawn_positions = Vec::with_capacity(map.encounters.len().max(4));
    let mut next_enemy_instance_id = 1u32;

    if map.encounters.is_empty() {
        out.push(GameEvent::Combat(CombatEvent::SetMapEnemies {
            enemies,
            respawn_positions,
            next_enemy_instance_id,
        }));
        return;
    }

    let encounter_enemy_indices: Vec<usize> = map
        .encounters
        .iter()
        .filter_map(|(id, _)| enemy_data.iter().position(|enemy| &enemy.id == id))
        .collect();
    if encounter_enemy_indices.is_empty() {
        out.push(GameEvent::Combat(CombatEvent::SetMapEnemies {
            enemies,
            respawn_positions,
            next_enemy_instance_id,
        }));
        return;
    }

    let mut enemy_tile_count = 0usize;
    for y in 0..map.height {
        for x in 0..map.width {
            if map.get_tile(x, y) != crate::data::Tile::Enemy {
                continue;
            }

            let idx = encounter_enemy_indices[enemy_tile_count % encounter_enemy_indices.len()];
            let Some(enemy) = enemy_data.get(idx) else {
                continue;
            };

            let (instance_id, next_id) = allocate_enemy_instance_id(next_enemy_instance_id);
            next_enemy_instance_id = next_id;
            enemies.push(FieldEnemy::new(enemy.clone(), x, y, instance_id));
            respawn_positions.push((x, y, idx));
            enemy_tile_count += 1;
        }
    }

    out.push(GameEvent::Combat(CombatEvent::SetMapEnemies {
        enemies,
        respawn_positions,
        next_enemy_instance_id: next_enemy_instance_id.max(1),
    }));
}

fn resolve_player_attack_action(
    state: &CombatState,
    player_x: usize,
    player_y: usize,
    player_atk: i32,
    facing: Direction,
    out: &mut Vec<GameEvent>,
) {
    if state.player_attack_cooldown > 0 {
        return;
    }

    let (tx, ty) = facing.apply(player_x, player_y);
    let mut skill_effects = state.skill_effects.clone();
    skill_effects.push(SkillEffect {
        x: tx,
        y: ty,
        effect_type: SkillType::Attack,
        timer: ATTACK_EFFECT_DURATION,
    });

    for enemy in &state.enemies {
        if enemy.x == tx && enemy.y == ty && enemy.hp > 0 {
            let damage = (player_atk - enemy.data.def / 2).max(1);
            let hp = (enemy.hp - damage).max(0);
            out.push(GameEvent::Combat(CombatEvent::EnemyHpSet {
                enemy_id: enemy.instance_id,
                hp,
            }));
            out.push(GameEvent::Combat(CombatEvent::EnemyHitFlashSet {
                enemy_id: enemy.instance_id,
                hit_flash: HIT_FLASH_DURATION,
            }));
            if hp <= 0 {
                out.push(GameEvent::Combat(CombatEvent::EnemyDespawn(
                    enemy.instance_id,
                )));
                out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                    enemy_id: enemy.data.id.clone(),
                    exp: enemy.data.exp,
                    gold: enemy.data.gold,
                }));
            }
            break;
        }
    }
    out.push(GameEvent::Combat(CombatEvent::SetPlayerAttackCooldown(
        PLAYER_ATTACK_COOLDOWN,
    )));
    out.push(GameEvent::Combat(CombatEvent::SetSkillEffects(
        skill_effects,
    )));
}

fn resolve_skill_action(
    state: &CombatState,
    skill: &Skill,
    player_x: usize,
    player_y: usize,
    player_atk: i32,
    facing: Direction,
    out: &mut Vec<GameEvent>,
) {
    let mut kills = Vec::with_capacity(4);
    let mut hp_updates: Vec<(u32, i32)> = Vec::with_capacity(8);
    let mut skill_effects = state.skill_effects.clone();
    let damage = skill.power + player_atk / 2;

    match skill.skill_type {
        SkillType::Attack => {}
        SkillType::Ranged => {
            for dist in 1..=skill.range {
                let (tx, ty) = facing.apply_distance(player_x, player_y, dist);
                skill_effects.push(SkillEffect {
                    x: tx,
                    y: ty,
                    effect_type: SkillType::Ranged,
                    timer: SKILL_EFFECT_DURATION,
                });
                if let Some(kill) = damage_enemy_at(state, tx, ty, damage, &mut hp_updates, out) {
                    kills.push(kill);
                    break;
                }
            }
        }
        SkillType::Area => {
            for dir in [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                let (tx, ty) = dir.apply(player_x, player_y);
                skill_effects.push(SkillEffect {
                    x: tx,
                    y: ty,
                    effect_type: SkillType::Area,
                    timer: SKILL_EFFECT_DURATION,
                });
                if let Some(kill) = damage_enemy_at(state, tx, ty, damage, &mut hp_updates, out) {
                    kills.push(kill);
                }
            }
        }
        SkillType::Heal => {}
    }

    let heal_amount = if skill.heal_power > 0 {
        skill_effects.push(SkillEffect {
            x: player_x,
            y: player_y,
            effect_type: SkillType::Heal,
            timer: HEAL_EFFECT_DURATION,
        });
        skill.heal_power
    } else {
        0
    };

    if heal_amount > 0 {
        out.push(GameEvent::Combat(CombatEvent::Heal(heal_amount)));
    }
    for reward in kills {
        out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
            enemy_id: reward.enemy_id,
            exp: reward.exp,
            gold: reward.gold,
        }));
    }
    out.push(GameEvent::Combat(CombatEvent::SetSkillEffects(
        skill_effects,
    )));
}

fn damage_enemy_at(
    state: &CombatState,
    x: usize,
    y: usize,
    damage: i32,
    hp_updates: &mut Vec<(u32, i32)>,
    out: &mut Vec<GameEvent>,
) -> Option<KillReward> {
    for enemy in &state.enemies {
        if enemy.x == x && enemy.y == y {
            let current_hp = hp_updates
                .iter()
                .find_map(|(id, hp)| (*id == enemy.instance_id).then_some(*hp))
                .unwrap_or(enemy.hp);
            if current_hp <= 0 {
                return None;
            }
            let actual_damage = (damage - enemy.data.def / 2).max(1);
            let hp = (current_hp - actual_damage).max(0);
            hp_updates.retain(|(id, _)| *id != enemy.instance_id);
            hp_updates.push((enemy.instance_id, hp));
            out.push(GameEvent::Combat(CombatEvent::EnemyHpSet {
                enemy_id: enemy.instance_id,
                hp,
            }));
            out.push(GameEvent::Combat(CombatEvent::EnemyHitFlashSet {
                enemy_id: enemy.instance_id,
                hit_flash: HIT_FLASH_DURATION,
            }));
            if hp <= 0 {
                out.push(GameEvent::Combat(CombatEvent::EnemyDespawn(
                    enemy.instance_id,
                )));
                return Some(KillReward {
                    enemy_id: enemy.data.id.clone(),
                    exp: enemy.data.exp,
                    gold: enemy.data.gold,
                });
            }
            return None;
        }
    }
    None
}

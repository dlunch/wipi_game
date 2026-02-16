use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::{Direction, Enemy, Map, Skill, SkillType};

use crate::game::state::{CombatState, FieldEnemy, KillReward, SkillEffect};
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{CombatEvent, GameEvent, GameState, SessionEvent, TransitionEvent};

const ENEMY_MOVE_INTERVAL: u32 = 8;
const MP_REGEN_INTERVAL: u32 = 60;
const HIT_FLASH_DURATION: u32 = 10;
const ENEMY_ATTACK_COOLDOWN: u32 = 30;
const PLAYER_ATTACK_COOLDOWN: u32 = 15;
const ATTACK_EFFECT_DURATION: u32 = 6;
const SKILL_EFFECT_DURATION: u32 = 8;
const HEAL_EFFECT_DURATION: u32 = 15;

enum CombatActionResult {
    Attack(Option<KillReward>),
    Skill(SkillActionResult),
}

struct SkillActionResult {
    heal_amount: i32,
    kills: Vec<KillReward>,
}

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
    events.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(
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
        events.push(GameEvent::Combat(CombatEvent::SetPlayerAttackCooldown(
            player_attack_cooldown,
        )));
    }
    if player_hit_flash > 0 {
        player_hit_flash = player_hit_flash.saturating_sub(1);
        events.push(GameEvent::Combat(CombatEvent::SetPlayerHitFlash(
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
        events.push(GameEvent::Combat(CombatEvent::SetSkillEffects(
            skill_effects.clone(),
        )));
    }

    let mut damage_taken = 0;

    if update_counter.is_multiple_of(ENEMY_MOVE_INTERVAL) {
        for enemy in &mut enemies {
            if !enemy_is_dead(enemy) {
                update_enemy(enemy, player_x, player_y, map);
            }
        }
    }

    let previous_enemies = state.enemies.clone();
    for enemy in &mut enemies {
        if enemy_is_dead(enemy) {
            continue;
        }

        if enemy_distance_to(enemy, player_x, player_y) <= 1 && enemy_can_attack(enemy) {
            let raw_damage = enemy_do_attack(enemy);
            let actual_damage = (raw_damage - player_def / 2).max(1);
            damage_taken += actual_damage;
            if player_hit_flash != 10 {
                player_hit_flash = 10;
                events.push(GameEvent::Combat(CombatEvent::SetPlayerHitFlash(
                    player_hit_flash,
                )));
            }
        }
    }

    enemies.retain(|enemy| !enemy_is_dead(enemy));

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
        events.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(
            respawn_timer,
        )));
    }
    if next_enemy_instance_id != state.next_enemy_instance_id {
        events.push(GameEvent::Combat(CombatEvent::SetNextEnemyInstanceId(
            next_enemy_instance_id,
        )));
    }

    tick_resource_state(skill_cooldowns, mp_regen_timer, &mut events);
    if damage_taken > 0 {
        events.push(GameEvent::Combat(CombatEvent::TakeDamage(damage_taken)));
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
        events.push(GameEvent::Combat(CombatEvent::SetSkillCooldowns(
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
        events.push(GameEvent::Combat(CombatEvent::SetMpRegenTimer(
            next_mp_regen_timer,
        )));
    }
    if recover_mp {
        events.push(GameEvent::Combat(CombatEvent::RecoverMp(1)));
    }
}

fn push_enemy_events(previous: &[FieldEnemy], next: &[FieldEnemy], events: &mut Vec<GameEvent>) {
    for enemy in previous {
        if !next
            .iter()
            .any(|next_enemy| next_enemy.instance_id == enemy.instance_id)
        {
            events.push(GameEvent::Combat(CombatEvent::EnemyDespawn(
                enemy.instance_id,
            )));
        }
    }

    for enemy in next {
        let Some(previous_enemy) = previous
            .iter()
            .find(|previous_enemy| previous_enemy.instance_id == enemy.instance_id)
        else {
            events.push(GameEvent::Combat(CombatEvent::EnemySpawn(enemy.clone())));
            continue;
        };

        if previous_enemy.x != enemy.x || previous_enemy.y != enemy.y {
            events.push(GameEvent::Combat(CombatEvent::EnemyMove {
                enemy_id: enemy.instance_id,
                x: enemy.x,
                y: enemy.y,
            }));
        }
        if previous_enemy.hp != enemy.hp {
            events.push(GameEvent::Combat(CombatEvent::EnemyHpSet {
                enemy_id: enemy.instance_id,
                hp: enemy.hp,
            }));
        }
        if previous_enemy.attack_cooldown != enemy.attack_cooldown {
            events.push(GameEvent::Combat(CombatEvent::EnemyAttackCooldownSet {
                enemy_id: enemy.instance_id,
                cooldown: enemy.attack_cooldown,
            }));
        }
        if previous_enemy.hit_flash != enemy.hit_flash {
            events.push(GameEvent::Combat(CombatEvent::EnemyHitFlashSet {
                enemy_id: enemy.instance_id,
                hit_flash: enemy.hit_flash,
            }));
        }
    }
}

struct UpdateCombatResolver;
struct CombatPlayerActionResolver;
struct CombatMapSyncResolver;

static UPDATE_COMBAT_RESOLVER: UpdateCombatResolver = UpdateCombatResolver;
static COMBAT_PLAYER_ACTION_RESOLVER: CombatPlayerActionResolver = CombatPlayerActionResolver;
static COMBAT_MAP_SYNC_RESOLVER: CombatMapSyncResolver = CombatMapSyncResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![
        &UPDATE_COMBAT_RESOLVER,
        &COMBAT_PLAYER_ACTION_RESOLVER,
        &COMBAT_MAP_SYNC_RESOLVER,
    ]
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
        let Some(map) = ctx.data().find_map(&s.leader.current_map_id) else {
            return Ok(Vec::new());
        };

        Ok(resolve_tick(
            &s.combat,
            s.leader.x,
            s.leader.y,
            s.leader.total_def(),
            (s.skill_cooldowns, s.mp_regen_timer),
            map,
            &ctx.data().enemies,
        ))
    }
}

impl DomainEventResolver for CombatPlayerActionResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::CombatPlayerAction(_))
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let GameEvent::CombatPlayerAction(action) = event else {
            return Ok(Vec::new());
        };
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        let mut next_combat = s.combat.clone();
        let mut events = Vec::new();

        if let Some((slot, skill)) = action.skill() {
            if !s
                .leader
                .can_use_skill(&s.skill_cooldowns, slot, skill.mp_cost)
            {
                return Ok(Vec::new());
            }

            let combat_event = apply_skill_action(
                &mut next_combat,
                skill,
                s.leader.x,
                s.leader.y,
                s.leader.total_atk(),
                s.leader.facing,
            );
            let CombatActionResult::Skill(result) = combat_event else {
                return Ok(Vec::new());
            };

            let mut next_skill_cooldowns = s.skill_cooldowns;
            next_skill_cooldowns[slot] = skill.cooldown;
            events.push(GameEvent::Combat(CombatEvent::SetSkillCooldowns(
                next_skill_cooldowns,
            )));
            events.push(GameEvent::Combat(CombatEvent::RecoverMp(-skill.mp_cost)));

            if result.heal_amount > 0 {
                events.push(GameEvent::Combat(CombatEvent::Heal(result.heal_amount)));
            }

            for reward in result.kills {
                events.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                    enemy_id: reward.enemy_id,
                    exp: reward.exp,
                    gold: reward.gold,
                }));
            }
        } else if let CombatActionResult::Attack(Some(reward)) = apply_player_attack_action(
            &mut next_combat,
            s.leader.x,
            s.leader.y,
            s.leader.total_atk(),
            s.leader.facing,
        ) {
            events.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id: reward.enemy_id,
                exp: reward.exp,
                gold: reward.gold,
            }));
        }

        events.push(GameEvent::Combat(CombatEvent::SetPlayerAttackCooldown(
            next_combat.player_attack_cooldown,
        )));
        events.push(GameEvent::Combat(CombatEvent::SetSkillEffects(
            next_combat.skill_effects.clone(),
        )));
        push_enemy_events(&s.combat.enemies, &next_combat.enemies, &mut events);

        Ok(events)
    }
}

impl DomainEventResolver for CombatMapSyncResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::Transition(TransitionEvent::MapChanged)
                | GameEvent::Session(SessionEvent::SpawnCurrentMapEnemies)
        )
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, _event: &GameEvent) -> Result<Vec<GameEvent>> {
        let session = ctx.session.ok_or_else(|| anyhow!("No active session"))?;
        let Some(map) = ctx.data().find_map(&session.leader.current_map_id) else {
            return Ok(Vec::new());
        };

        let (enemies, respawn_positions, next_enemy_instance_id) =
            build_map_enemies(map, &ctx.data().enemies);
        Ok(vec![GameEvent::Combat(CombatEvent::SetMapEnemies {
            enemies,
            respawn_positions,
            next_enemy_instance_id,
        })])
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

fn enemy_is_dead(enemy: &FieldEnemy) -> bool {
    enemy.hp <= 0
}

fn enemy_take_damage(enemy: &mut FieldEnemy, damage: i32) {
    enemy.hp = (enemy.hp - damage).max(0);
    enemy.hit_flash = HIT_FLASH_DURATION;
}

fn enemy_distance_to(enemy: &FieldEnemy, px: usize, py: usize) -> usize {
    enemy.x.abs_diff(px) + enemy.y.abs_diff(py)
}

fn update_enemy(enemy: &mut FieldEnemy, player_x: usize, player_y: usize, map: &Map) {
    if enemy.hit_flash > 0 {
        enemy.hit_flash -= 1;
    }
    if enemy.attack_cooldown > 0 {
        enemy.attack_cooldown -= 1;
    }

    if enemy_distance_to(enemy, player_x, player_y) > 1 {
        move_enemy_towards(enemy, player_x, player_y, map);
    }
}

fn move_enemy_towards(enemy: &mut FieldEnemy, target_x: usize, target_y: usize, map: &Map) {
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

    let new_x = enemy.x.checked_add_signed(dx as isize);
    let new_y = enemy.y.checked_add_signed(dy as isize);

    if let Some(nx) = new_x
        && dx != 0
        && map.get_tile(nx, enemy.y).is_passable()
    {
        enemy.x = nx;
        return;
    }
    if let Some(ny) = new_y
        && dy != 0
        && map.get_tile(enemy.x, ny).is_passable()
    {
        enemy.y = ny;
    }
}

fn enemy_can_attack(enemy: &FieldEnemy) -> bool {
    enemy.attack_cooldown == 0
}

fn enemy_do_attack(enemy: &mut FieldEnemy) -> i32 {
    enemy.attack_cooldown = ENEMY_ATTACK_COOLDOWN;
    enemy.data.atk
}

fn build_map_enemies(
    map: &Map,
    enemy_data: &[Enemy],
) -> (Vec<FieldEnemy>, Vec<(usize, usize, usize)>, u32) {
    let mut enemies = Vec::new();
    let mut respawn_positions = Vec::new();
    let mut enemy_tiles: Vec<(usize, usize)> = Vec::new();
    let mut next_enemy_instance_id = 1u32;

    for y in 0..map.height {
        for x in 0..map.width {
            if map.get_tile(x, y) == crate::data::Tile::Enemy {
                enemy_tiles.push((x, y));
            }
        }
    }

    if enemy_tiles.is_empty() || map.encounters.is_empty() {
        return (enemies, respawn_positions, next_enemy_instance_id);
    }

    let available_enemies: Vec<&Enemy> = map
        .encounters
        .iter()
        .filter_map(|(id, _)| enemy_data.iter().find(|enemy| &enemy.id == id))
        .collect();
    if available_enemies.is_empty() {
        return (enemies, respawn_positions, next_enemy_instance_id);
    }

    for (i, (x, y)) in enemy_tiles.iter().enumerate() {
        let enemy_idx = i % available_enemies.len();
        let enemy = available_enemies[enemy_idx];
        let instance_id = allocate_enemy_instance_id(&mut next_enemy_instance_id);
        enemies.push(FieldEnemy::new(enemy.clone(), *x, *y, instance_id));
        respawn_positions.push((*x, *y, enemy_idx));
    }

    (enemies, respawn_positions, next_enemy_instance_id.max(1))
}

fn apply_player_attack_action(
    state: &mut CombatState,
    player_x: usize,
    player_y: usize,
    player_atk: i32,
    facing: Direction,
) -> CombatActionResult {
    if state.player_attack_cooldown > 0 {
        return CombatActionResult::Attack(None);
    }

    let (tx, ty) = facing.apply(player_x, player_y);
    state.skill_effects.push(SkillEffect {
        x: tx,
        y: ty,
        effect_type: SkillType::Attack,
        timer: ATTACK_EFFECT_DURATION,
    });

    let mut kill = None;
    for enemy in &mut state.enemies {
        if enemy.x == tx && enemy.y == ty && !enemy_is_dead(enemy) {
            let damage = (player_atk - enemy.data.def / 2).max(1);
            enemy_take_damage(enemy, damage);
            if enemy_is_dead(enemy) {
                kill = Some(KillReward {
                    enemy_id: enemy.data.id.clone(),
                    exp: enemy.data.exp,
                    gold: enemy.data.gold,
                });
            }
            break;
        }
    }
    state.player_attack_cooldown = PLAYER_ATTACK_COOLDOWN;
    CombatActionResult::Attack(kill)
}

fn apply_skill_action(
    state: &mut CombatState,
    skill: &Skill,
    player_x: usize,
    player_y: usize,
    player_atk: i32,
    facing: Direction,
) -> CombatActionResult {
    let mut kills = Vec::new();
    let damage = skill.power + player_atk / 2;

    match skill.skill_type {
        SkillType::Attack => {}
        SkillType::Ranged => {
            for dist in 1..=skill.range {
                let (tx, ty) = facing.apply_distance(player_x, player_y, dist);
                state.skill_effects.push(SkillEffect {
                    x: tx,
                    y: ty,
                    effect_type: SkillType::Ranged,
                    timer: SKILL_EFFECT_DURATION,
                });
                if let Some(kill) = damage_enemy_at(state, tx, ty, damage) {
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
                state.skill_effects.push(SkillEffect {
                    x: tx,
                    y: ty,
                    effect_type: SkillType::Area,
                    timer: SKILL_EFFECT_DURATION,
                });
                if let Some(kill) = damage_enemy_at(state, tx, ty, damage) {
                    kills.push(kill);
                }
            }
        }
        SkillType::Heal => {}
    }

    let heal_amount = if skill.heal_power > 0 {
        state.skill_effects.push(SkillEffect {
            x: player_x,
            y: player_y,
            effect_type: SkillType::Heal,
            timer: HEAL_EFFECT_DURATION,
        });
        skill.heal_power
    } else {
        0
    };

    CombatActionResult::Skill(SkillActionResult { heal_amount, kills })
}

fn damage_enemy_at(state: &mut CombatState, x: usize, y: usize, damage: i32) -> Option<KillReward> {
    for enemy in &mut state.enemies {
        if enemy.x == x && enemy.y == y && !enemy_is_dead(enemy) {
            let actual_damage = (damage - enemy.data.def / 2).max(1);
            enemy_take_damage(enemy, actual_damage);
            if enemy_is_dead(enemy) {
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

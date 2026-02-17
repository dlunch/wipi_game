use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{Direction, Enemy, Map, Skill, SkillType, Tile};
use crate::game::state::{
    AllyCombatantState, CombatStatsSnapshot, CombatantState, EnemyCombatantState, EntityKind,
    EntityStat, EntityState, TimedKind, TimedState,
};
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{
    CombatEvent, GameData, GameEvent, GameEventKind, TransitionEvent, WorldEvent, WorldState,
};

const ENEMY_MOVE_INTERVAL: u32 = 8;
const MP_REGEN_INTERVAL: u32 = 60;
const ENEMY_ATTACK_COOLDOWN: u32 = 30;
const PLAYER_ATTACK_COOLDOWN: u32 = 15;
const STATUS_TICK_INTERVAL: u32 = 20;
const POISON_DAMAGE: i32 = 2;
const POISON_DURATION: u32 = 180;
const ARMOR_BREAK_DURATION: u32 = 120;

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
        data: &Rc<GameData>,
        world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::UpdateCombat => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                resolve_tick(data, world, out);
            }
            GameEvent::CombatPlayerAction(action) => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                resolve_player_action(data, world, action, out);
            }
            GameEvent::Transition(TransitionEvent::MapChanged) => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                resolve_map_changed(data, world, out);
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_tick(data: &GameData, world: &WorldState, out: &mut Vec<GameEvent>) {
    if !world.combat.active {
        return;
    }
    let Some(leader_id) = world.leader_id() else {
        return;
    };
    let Some(leader_entity) = world.leader_entity() else {
        return;
    };
    let Some(leader_combatant) = world.combat.combatant(leader_id) else {
        return;
    };
    let Some(map) = data.find_map(&leader_entity.map_id) else {
        return;
    };

    let next_counter = world.combat.update_counter.wrapping_add(1);
    out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(
        next_counter,
    )));

    tick_combatant_timed(leader_id, leader_combatant, next_counter, out);

    let mut occupied_positions = Vec::with_capacity(world.combat.enemies.len() + 1);
    occupied_positions.push((leader_entity.x, leader_entity.y));
    for enemy in &world.combat.enemies {
        if let Some(entity) = world.entity(enemy.entity_id) {
            occupied_positions.push((entity.x, entity.y));
        }
    }

    let mut total_player_damage = 0;
    for enemy in &world.combat.enemies {
        if enemy.combatant.stats.current_hp <= 0 {
            out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(enemy.entity_id)));
            continue;
        }

        tick_combatant_timed(enemy.entity_id, &enemy.combatant, next_counter, out);
        maybe_enemy_poison_tick(data, enemy, next_counter, out);

        let enemy_stunned = enemy.combatant.timed.time_left(TimedKind::Stun) > 0;
        let mut attack_cooldown = enemy.combatant.timed.time_left(TimedKind::AttackCooldown);
        if attack_cooldown > 0 {
            attack_cooldown -= 1;
            out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: enemy.entity_id,
                kind: TimedKind::AttackCooldown,
                time_left: attack_cooldown,
            }));
        }

        let Some(enemy_entity) = world.entity(enemy.entity_id) else {
            continue;
        };
        let mut next_x = enemy_entity.x;
        let mut next_y = enemy_entity.y;

        if !enemy_stunned && next_counter.is_multiple_of(ENEMY_MOVE_INTERVAL) {
            let (mx, my) = next_enemy_position_towards(
                enemy_entity.x,
                enemy_entity.y,
                leader_entity.x,
                leader_entity.y,
                map,
                occupied_positions.as_slice(),
            );
            next_x = mx;
            next_y = my;
        }
        if next_x != enemy_entity.x || next_y != enemy_entity.y {
            out.push(GameEvent::Combat(CombatEvent::MoveEnemy {
                entity_id: enemy.entity_id,
                x: next_x,
                y: next_y,
            }));
            if let Some(slot) = occupied_positions
                .iter_mut()
                .find(|(x, y)| *x == enemy_entity.x && *y == enemy_entity.y)
            {
                slot.0 = next_x;
                slot.1 = next_y;
            }
        }

        if !enemy_stunned
            && enemy_distance(next_x, next_y, leader_entity.x, leader_entity.y) <= 1
            && attack_cooldown == 0
            && leader_combatant.stats.current_hp > 0
        {
            let Some(enemy_data) = data.find_enemy(&enemy.source_enemy_id) else {
                continue;
            };
            let effective_def = if leader_combatant.timed.time_left(TimedKind::ArmorBreak) > 0 {
                leader_combatant.stats.def / 2
            } else {
                leader_combatant.stats.def
            };
            let damage = enemy_data.atk.saturating_sub(effective_def / 2).max(1);
            total_player_damage += damage;
            out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: enemy.entity_id,
                kind: TimedKind::AttackCooldown,
                time_left: ENEMY_ATTACK_COOLDOWN,
            }));
            out.push(GameEvent::Combat(CombatEvent::SetEntityHitFlash {
                entity_id: leader_id,
                timer: 10,
            }));
        }
    }

    if total_player_damage > 0 {
        out.push(GameEvent::Combat(CombatEvent::TakeDamage {
            entity_id: leader_id,
            amount: total_player_damage,
        }));
    }
}

fn tick_combatant_timed(
    entity_id: u32,
    combatant: &CombatantState,
    next_counter: u32,
    out: &mut Vec<GameEvent>,
) {
    for effect in &combatant.timed.effects {
        match effect.kind {
            TimedKind::MpRegenTick => {
                let next = if effect.time_left <= 1 {
                    out.push(GameEvent::Combat(CombatEvent::RecoverMp {
                        entity_id,
                        amount: 1,
                    }));
                    MP_REGEN_INTERVAL
                } else {
                    effect.time_left - 1
                };
                out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                    entity_id,
                    kind: TimedKind::MpRegenTick,
                    time_left: next,
                }));
            }
            TimedKind::Poison => {
                let next = effect.time_left.saturating_sub(1);
                out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                    entity_id,
                    kind: TimedKind::Poison,
                    time_left: next,
                }));
                if effect.time_left > 0 && next_counter.is_multiple_of(STATUS_TICK_INTERVAL) {
                    out.push(GameEvent::Combat(CombatEvent::TakeDamage {
                        entity_id,
                        amount: POISON_DAMAGE,
                    }));
                }
            }
            TimedKind::Stun
            | TimedKind::ArmorBreak
            | TimedKind::AttackCooldown
            | TimedKind::SkillCooldown(_) => {
                if effect.time_left > 0 {
                    out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                        entity_id,
                        kind: effect.kind,
                        time_left: effect.time_left - 1,
                    }));
                }
            }
        }
    }
}

fn maybe_enemy_poison_tick(
    data: &GameData,
    enemy: &EnemyCombatantState,
    next_counter: u32,
    out: &mut Vec<GameEvent>,
) {
    if enemy.combatant.timed.time_left(TimedKind::Poison) == 0 {
        return;
    }
    if !next_counter.is_multiple_of(STATUS_TICK_INTERVAL) {
        return;
    }
    let mut next_stats = enemy.combatant.stats;
    next_stats.current_hp = next_stats.current_hp.saturating_sub(POISON_DAMAGE).max(0);
    out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
        entity_id: enemy.entity_id,
        stats: next_stats,
    }));
    if next_stats.current_hp <= 0 {
        out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(enemy.entity_id)));
        if let Some(enemy_data) = data.find_enemy(&enemy.source_enemy_id) {
            out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id: enemy_data.id.clone(),
                exp: enemy_data.exp,
                gold: enemy_data.gold,
            }));
        }
    }
}

fn resolve_player_action(
    data: &GameData,
    world: &WorldState,
    action: &crate::game::ExploreAction,
    out: &mut Vec<GameEvent>,
) {
    let Some(leader_id) = world.leader_id() else {
        return;
    };
    let Some(leader_entity) = world.leader_entity() else {
        return;
    };
    let Some(leader_combatant) = world.combat.combatant(leader_id) else {
        return;
    };
    if leader_combatant.stats.current_hp <= 0 {
        return;
    }
    if leader_combatant.timed.time_left(TimedKind::Stun) > 0 {
        return;
    }

    if let Some((slot, skill)) = action.skill() {
        if leader_combatant
            .timed
            .time_left(TimedKind::SkillCooldown(slot as u8))
            > 0
        {
            return;
        }
        if leader_combatant.stats.current_mp < skill.mp_cost {
            return;
        }
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: TimedKind::SkillCooldown(slot as u8),
            time_left: skill.cooldown,
        }));
        out.push(GameEvent::Combat(CombatEvent::RecoverMp {
            entity_id: leader_id,
            amount: -skill.mp_cost,
        }));
        resolve_skill_action(data, world, leader_entity, leader_combatant, skill, out);
        return;
    }

    if leader_combatant.timed.time_left(TimedKind::AttackCooldown) > 0 {
        return;
    }
    let (tx, ty) = leader_entity.facing.apply(leader_entity.x, leader_entity.y);
    damage_enemy_at_position(data, world, tx, ty, leader_combatant.stats.atk, None, out);
    out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
        entity_id: leader_id,
        kind: TimedKind::AttackCooldown,
        time_left: PLAYER_ATTACK_COOLDOWN,
    }));
}

fn resolve_skill_action(
    data: &GameData,
    world: &WorldState,
    leader: &EntityState,
    leader_combatant: &CombatantState,
    skill: &Skill,
    out: &mut Vec<GameEvent>,
) {
    let base_damage = skill.power + leader_combatant.stats.atk / 2;
    match skill.skill_type {
        SkillType::Ranged => {
            for dist in 1..=skill.range {
                let (tx, ty) = leader.facing.apply_distance(leader.x, leader.y, dist);
                if damage_enemy_at_position(
                    data,
                    world,
                    tx,
                    ty,
                    base_damage,
                    Some((TimedKind::Poison, POISON_DURATION / 2)),
                    out,
                ) {
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
                let (tx, ty) = dir.apply(leader.x, leader.y);
                let _ = damage_enemy_at_position(
                    data,
                    world,
                    tx,
                    ty,
                    base_damage,
                    Some((TimedKind::ArmorBreak, ARMOR_BREAK_DURATION / 2)),
                    out,
                );
            }
        }
        SkillType::Heal => {
            if let Some(leader_id) = world.leader_id() {
                out.push(GameEvent::Combat(CombatEvent::Heal {
                    entity_id: leader_id,
                    amount: skill.heal_power.max(0),
                }));
            }
        }
    }
}

fn damage_enemy_at_position(
    data: &GameData,
    world: &WorldState,
    x: usize,
    y: usize,
    raw_damage: i32,
    timed_effect: Option<(TimedKind, u32)>,
    out: &mut Vec<GameEvent>,
) -> bool {
    for enemy in &world.combat.enemies {
        let Some(entity) = world.entity(enemy.entity_id) else {
            continue;
        };
        if entity.x != x || entity.y != y {
            continue;
        }
        let mut next_stats = enemy.combatant.stats;
        let effective_def = if enemy.combatant.timed.time_left(TimedKind::ArmorBreak) > 0 {
            next_stats.def / 2
        } else {
            next_stats.def
        };
        let damage = raw_damage.saturating_sub(effective_def / 2).max(1);
        next_stats.current_hp = next_stats.current_hp.saturating_sub(damage).max(0);

        out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
            entity_id: enemy.entity_id,
            stats: next_stats,
        }));
        out.push(GameEvent::Combat(CombatEvent::EnemyHitFlashSet {
            entity_id: enemy.entity_id,
            hit_flash: 10,
        }));

        if let Some((kind, duration)) = timed_effect
            && enemy.combatant.timed.time_left(kind) < duration
        {
            out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: enemy.entity_id,
                kind,
                time_left: duration,
            }));
        }

        if next_stats.current_hp <= 0 {
            out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(enemy.entity_id)));
            if let Some(enemy_data) = data.find_enemy(&enemy.source_enemy_id) {
                out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                    enemy_id: enemy_data.id.clone(),
                    exp: enemy_data.exp,
                    gold: enemy_data.gold,
                }));
            }
        }
        return true;
    }
    false
}

fn resolve_map_changed(data: &GameData, world: &WorldState, out: &mut Vec<GameEvent>) {
    for enemy in &world.combat.enemies {
        out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(enemy.entity_id)));
    }

    let Some(leader_id) = world.leader_id() else {
        return;
    };
    let Some(leader_entity) = world.leader_entity() else {
        return;
    };
    let Some(map) = data.find_map(&leader_entity.map_id) else {
        return;
    };

    let Some(leader_stats) = world
        .combat
        .combatant(leader_id)
        .map(|combatant| combatant.stats)
    else {
        return;
    };
    out.push(GameEvent::Combat(CombatEvent::SetAllies(vec![
        AllyCombatantState {
            entity_id: leader_id,
            combatant: CombatantState {
                stats: leader_stats,
                timed: world
                    .combat
                    .combatant(leader_id)
                    .map(|combatant| combatant.timed.clone())
                    .unwrap_or_default(),
            },
        },
    ])));

    let enemy_templates: Vec<&Enemy> = map
        .encounters
        .iter()
        .filter_map(|(enemy_id, _)| data.find_enemy(enemy_id))
        .collect();
    if enemy_templates.is_empty() || map.peaceful {
        out.push(GameEvent::Combat(CombatEvent::SetEnemies(Vec::new())));
        out.push(GameEvent::Combat(CombatEvent::SetActive(!map.peaceful)));
        out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
        out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(0)));
        return;
    }

    let mut enemies = Vec::new();
    let mut next_entity_id = world.entities.next_entity_id.max(1);
    let mut spawn_index = 0usize;
    for y in 0..map.height {
        for x in 0..map.width {
            if map.get_tile(x, y) != Tile::Enemy {
                continue;
            }
            let enemy_data = enemy_templates[spawn_index % enemy_templates.len()];
            spawn_index += 1;

            let entity_id = next_entity_id;
            next_entity_id = next_entity_id.wrapping_add(1).max(1);
            let enemy_entity = EntityState {
                id: entity_id,
                kind: EntityKind::Enemy,
                name: enemy_data.name.clone(),
                map_id: map.id.clone(),
                x,
                y,
                facing: Direction::Down,
                stat: EntityStat {
                    level: 1,
                    exp: 0,
                    exp_to_next: 100,
                    base_max_hp: enemy_data.hp,
                    base_max_mp: 0,
                    base_atk: enemy_data.atk,
                    base_def: enemy_data.def,
                },
                inventory: Vec::new(),
                loadout: Default::default(),
            };
            out.push(GameEvent::World(WorldEvent::UpsertEntity(enemy_entity)));
            enemies.push(EnemyCombatantState {
                entity_id,
                source_enemy_id: enemy_data.id.clone(),
                combatant: CombatantState {
                    stats: CombatStatsSnapshot {
                        max_hp: enemy_data.hp,
                        current_hp: enemy_data.hp,
                        max_mp: 0,
                        current_mp: 0,
                        atk: enemy_data.atk,
                        def: enemy_data.def,
                    },
                    timed: TimedState::default(),
                },
            });
        }
    }

    out.push(GameEvent::Combat(CombatEvent::SetEnemies(enemies)));
    out.push(GameEvent::Combat(CombatEvent::SetActive(true)));
    out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
    out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(0)));
}

fn next_enemy_position_towards(
    enemy_x: usize,
    enemy_y: usize,
    target_x: usize,
    target_y: usize,
    map: &Map,
    occupied: &[(usize, usize)],
) -> (usize, usize) {
    let dx: i32 = match target_x.cmp(&enemy_x) {
        core::cmp::Ordering::Greater => 1,
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
    };
    let dy: i32 = match target_y.cmp(&enemy_y) {
        core::cmp::Ordering::Greater => 1,
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
    };

    if dx != 0 {
        let nx = (enemy_x as i32 + dx).max(0) as usize;
        if map.get_tile(nx, enemy_y).is_passable()
            && !occupied.iter().any(|(x, y)| *x == nx && *y == enemy_y)
        {
            return (nx, enemy_y);
        }
    }
    if dy != 0 {
        let ny = (enemy_y as i32 + dy).max(0) as usize;
        if map.get_tile(enemy_x, ny).is_passable()
            && !occupied.iter().any(|(x, y)| *x == enemy_x && *y == ny)
        {
            return (enemy_x, ny);
        }
    }
    (enemy_x, enemy_y)
}

fn enemy_distance(x: usize, y: usize, px: usize, py: usize) -> usize {
    x.abs_diff(px) + y.abs_diff(py)
}

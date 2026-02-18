use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{Direction, Enemy, Map, Skill, SkillType, Tile};
use crate::game::state::{CombatStatsSnapshot, CombatantState, EntityKind, EntityState, TimedKind};
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{
    CombatEvent, EntityEvent, GameData, GameEvent, GameEventKind, TransitionEvent, WorldState,
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
        if !matches!(
            event,
            GameEvent::UpdateCombat
                | GameEvent::CombatPlayerAction(_)
                | GameEvent::Transition(TransitionEvent::MapChanged)
        ) {
            return Ok(());
        }

        let world = world.ok_or_else(|| anyhow!("No active world"))?;
        match event {
            GameEvent::UpdateCombat => resolve_tick(data, world, out)?,
            GameEvent::CombatPlayerAction(action) => {
                resolve_player_action(data, world, action, out)?
            }
            GameEvent::Transition(TransitionEvent::MapChanged) => {
                resolve_map_changed(data, world, out)?
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_tick(data: &GameData, world: &WorldState, out: &mut Vec<GameEvent>) -> Result<()> {
    if !world.combat.active {
        return Ok(());
    }
    let leader_id = world.leader_id()?;
    let leader_entity = world.leader_entity()?;
    let leader_combatant = world.combat.combatant(leader_id)?;
    let map = data.find_map(&leader_entity.map_id)?;

    let next_counter = world.combat.update_counter.wrapping_add(1);
    out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(
        next_counter,
    )));

    tick_combatant_timed(leader_id, leader_combatant, next_counter, out);

    let mut occupied_tiles = Vec::with_capacity(world.combat.enemies.len() + 1);
    occupied_tiles.push(tile_index(leader_entity.x, leader_entity.y, map.width));
    for enemy in &world.combat.enemies {
        let entity = world.entity(enemy.entity_id)?;
        occupied_tiles.push(tile_index(entity.x, entity.y, map.width));
    }

    let mut total_player_damage = 0;
    for enemy in &world.combat.enemies {
        if enemy.combatant.stats.current_hp <= 0 {
            out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(enemy.entity_id)));
            continue;
        }

        tick_combatant_timed(enemy.entity_id, &enemy.combatant, next_counter, out);

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

        let enemy_entity = world.entity(enemy.entity_id)?;
        let mut next_x = enemy_entity.x;
        let mut next_y = enemy_entity.y;

        if !enemy_stunned && next_counter.is_multiple_of(ENEMY_MOVE_INTERVAL) {
            let (mx, my) = next_enemy_position_towards(
                enemy_entity.x,
                enemy_entity.y,
                leader_entity.x,
                leader_entity.y,
                map,
                occupied_tiles.as_slice(),
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
            let old_tile = tile_index(enemy_entity.x, enemy_entity.y, map.width);
            if let Some(index) = occupied_tiles.iter().position(|tile| *tile == old_tile) {
                occupied_tiles.swap_remove(index);
            }
            occupied_tiles.push(tile_index(next_x, next_y, map.width));
        }

        if !enemy_stunned
            && enemy_distance(next_x, next_y, leader_entity.x, leader_entity.y) <= 1
            && attack_cooldown == 0
            && leader_combatant.stats.current_hp > 0
        {
            let enemy_data = data.find_enemy(&enemy.source_enemy_id)?;
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
        }
    }

    if total_player_damage > 0 {
        out.push(GameEvent::Combat(CombatEvent::TakeDamage {
            entity_id: leader_id,
            amount: total_player_damage,
        }));
    }
    Ok(())
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

fn resolve_player_action(
    data: &GameData,
    world: &WorldState,
    action: &crate::game::ExploreAction,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader_id = world.leader_id()?;
    let leader_entity = world.leader_entity()?;
    let leader_combatant = world.combat.combatant(leader_id)?;
    if leader_combatant.stats.current_hp <= 0
        || leader_combatant.timed.time_left(TimedKind::Stun) > 0
    {
        return Ok(());
    }

    if let Some((slot, skill)) = action.skill() {
        if leader_combatant
            .timed
            .time_left(TimedKind::SkillCooldown(slot as u8))
            > 0
        {
            return Ok(());
        }
        if leader_combatant.stats.current_mp < skill.mp_cost {
            return Ok(());
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
        resolve_skill_action(
            data,
            world,
            leader_id,
            leader_entity,
            leader_combatant,
            skill,
            out,
        )?;
    } else {
        if leader_combatant.timed.time_left(TimedKind::AttackCooldown) > 0 {
            return Ok(());
        }
        let (tx, ty) = leader_entity.facing.apply(leader_entity.x, leader_entity.y);
        let _ =
            damage_enemy_at_position(data, world, tx, ty, leader_combatant.stats.atk, None, out)?;
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: TimedKind::AttackCooldown,
            time_left: PLAYER_ATTACK_COOLDOWN,
        }));
    }
    Ok(())
}

fn resolve_skill_action(
    data: &GameData,
    world: &WorldState,
    leader_id: u32,
    leader: &EntityState,
    leader_combatant: &CombatantState,
    skill: &Skill,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
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
                )? {
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
                )?;
            }
        }
        SkillType::Heal => {
            out.push(GameEvent::Combat(CombatEvent::Heal {
                entity_id: leader_id,
                amount: skill.heal_power.max(0),
            }));
        }
    }
    Ok(())
}

fn damage_enemy_at_position(
    data: &GameData,
    world: &WorldState,
    x: usize,
    y: usize,
    raw_damage: i32,
    timed_effect: Option<(TimedKind, u32)>,
    out: &mut Vec<GameEvent>,
) -> Result<bool> {
    for enemy in &world.combat.enemies {
        let entity = world.entity(enemy.entity_id)?;
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

        out.push(GameEvent::Combat(CombatEvent::SetCombatantCurrentHp {
            entity_id: enemy.entity_id,
            current_hp: next_stats.current_hp,
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
            let enemy_data = data.find_enemy(&enemy.source_enemy_id)?;
            out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id: enemy_data.id.clone(),
                exp: enemy_data.exp,
                gold: enemy_data.gold,
            }));
        }
        return Ok(true);
    }
    Ok(false)
}

fn resolve_map_changed(
    data: &GameData,
    world: &WorldState,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    for enemy in &world.combat.enemies {
        out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(enemy.entity_id)));
    }

    let leader_id = world.leader_id()?;
    let leader_entity = world.leader_entity()?;
    let map = data.find_map(&leader_entity.map_id)?;

    let leader_combatant = world.combat.combatant(leader_id)?;
    emit_combat_stats(leader_id, &leader_combatant.stats, out);
    for effect in &leader_combatant.timed.effects {
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: effect.kind,
            time_left: effect.time_left,
        }));
    }

    let mut enemy_templates: Vec<&Enemy> = Vec::with_capacity(map.encounters.len());
    for (enemy_id, _) in &map.encounters {
        enemy_templates.push(data.find_enemy(enemy_id)?);
    }
    if enemy_templates.is_empty() || map.peaceful {
        out.push(GameEvent::Combat(CombatEvent::ClearEnemies));
        out.push(GameEvent::Combat(CombatEvent::SetActive(!map.peaceful)));
        out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
        out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(0)));
        return Ok(());
    }

    out.push(GameEvent::Combat(CombatEvent::ClearEnemies));
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
            out.push(GameEvent::Entity(EntityEvent::CreateEntity {
                entity_id,
                kind: EntityKind::Enemy,
                name: enemy_data.name.clone(),
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityTransform {
                entity_id,
                map_id: Some(map.id.clone()),
                position: Some((x, y)),
                facing: Some(Direction::Down),
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityLevel {
                entity_id,
                level: 1,
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityExp {
                entity_id,
                exp: 0,
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityExpToNext {
                entity_id,
                exp_to_next: 100,
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityBaseMaxHp {
                entity_id,
                base_max_hp: enemy_data.hp,
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityBaseMaxMp {
                entity_id,
                base_max_mp: 0,
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityBaseAtk {
                entity_id,
                base_atk: enemy_data.atk,
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityBaseDef {
                entity_id,
                base_def: enemy_data.def,
            }));
            out.push(GameEvent::Entity(EntityEvent::ClearEntityInventory {
                entity_id,
            }));
            let stats = CombatStatsSnapshot {
                max_hp: enemy_data.hp,
                current_hp: enemy_data.hp,
                max_mp: 0,
                current_mp: 0,
                atk: enemy_data.atk,
                def: enemy_data.def,
            };
            emit_combat_stats(entity_id, &stats, out);
        }
    }

    out.push(GameEvent::Combat(CombatEvent::SetActive(true)));
    out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
    out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(0)));
    Ok(())
}

fn next_enemy_position_towards(
    enemy_x: usize,
    enemy_y: usize,
    target_x: usize,
    target_y: usize,
    map: &Map,
    occupied: &[usize],
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
            && !occupied.contains(&tile_index(nx, enemy_y, map.width))
        {
            return (nx, enemy_y);
        }
    }
    if dy != 0 {
        let ny = (enemy_y as i32 + dy).max(0) as usize;
        if map.get_tile(enemy_x, ny).is_passable()
            && !occupied.contains(&tile_index(enemy_x, ny, map.width))
        {
            return (enemy_x, ny);
        }
    }
    (enemy_x, enemy_y)
}

fn tile_index(x: usize, y: usize, width: usize) -> usize {
    y * width + x
}

fn enemy_distance(x: usize, y: usize, px: usize, py: usize) -> usize {
    x.abs_diff(px) + y.abs_diff(py)
}

fn emit_combat_stats(entity_id: u32, stats: &CombatStatsSnapshot, out: &mut Vec<GameEvent>) {
    out.push(GameEvent::Combat(CombatEvent::SetCombatantMaxHp {
        entity_id,
        max_hp: stats.max_hp,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantCurrentHp {
        entity_id,
        current_hp: stats.current_hp,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantMaxMp {
        entity_id,
        max_mp: stats.max_mp,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantCurrentMp {
        entity_id,
        current_mp: stats.current_mp,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantAtk {
        entity_id,
        atk: stats.atk,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantDef {
        entity_id,
        def: stats.def,
    }));
}

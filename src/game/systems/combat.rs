use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{Direction, Enemy, Map, Skill, SkillType, Tile};
use crate::game::state::{CombatantState, EntityKind, EntityState, TimedKind, combat_attack_def};
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
            GameEventKind::Tick,
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
            GameEvent::Tick => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                resolve_tick(data, world, out)?
            }
            GameEvent::CombatPlayerAction(action) => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                resolve_player_action(data, world, action, out)?
            }
            GameEvent::Transition(TransitionEvent::MapChanged) => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                resolve_map_changed(data, world, out)?
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_tick(data: &GameData, world: &WorldState, out: &mut Vec<GameEvent>) -> Result<()> {
    let leader_id = world.leader_id()?;
    let leader_combatant = world.combat.combatant(leader_id)?;
    let leader_entity = world.leader_entity()?;
    let next_tick = world.tick_counter.wrapping_add(1);
    tick_mp_regen(leader_id, leader_entity, leader_combatant, next_tick, out);
    if !world.combat.active {
        return Ok(());
    }
    let map = data.find_map(&leader_entity.map_id)?;
    let (_, leader_def) = combat_attack_def(data, leader_entity)?;

    tick_combatant_effects(leader_id, leader_combatant, next_tick, out);

    let mut occupied_tiles = Vec::with_capacity(world.combat.enemies.len() + 1);
    occupied_tiles.push(tile_index(leader_entity.x, leader_entity.y, map.width));
    for enemy in &world.combat.enemies {
        let entity = world.entity(enemy.entity_id)?;
        occupied_tiles.push(tile_index(entity.x, entity.y, map.width));
    }

    let mut total_player_damage = 0;
    for enemy in &world.combat.enemies {
        let enemy_entity = world.entity(enemy.entity_id)?;
        if enemy_entity.current_hp <= 0 {
            out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(enemy.entity_id)));
            continue;
        }

        tick_combatant_effects(enemy.entity_id, &enemy.combatant, next_tick, out);

        let enemy_stunned = enemy.combatant.timed.is_active(TimedKind::Stun, next_tick);
        let attack_cooldown = enemy
            .combatant
            .timed
            .time_left(TimedKind::AttackCooldown, next_tick);

        let mut next_x = enemy_entity.x;
        let mut next_y = enemy_entity.y;

        if !enemy_stunned && next_tick.is_multiple_of(ENEMY_MOVE_INTERVAL) {
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
            && leader_entity.current_hp > 0
        {
            let enemy_data = data.find_enemy(&enemy.source_enemy_id)?;
            let effective_def = if leader_combatant
                .timed
                .is_active(TimedKind::ArmorBreak, next_tick)
            {
                leader_def / 2
            } else {
                leader_def
            };
            let damage = enemy_data.atk.saturating_sub(effective_def / 2).max(1);
            total_player_damage += damage;
            out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: enemy.entity_id,
                kind: TimedKind::AttackCooldown,
                end_tick: next_tick.wrapping_add(ENEMY_ATTACK_COOLDOWN),
            }));
        }
    }

    if total_player_damage > 0 {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityHp {
            entity_id: leader_id,
            delta: -total_player_damage,
        }));
    }
    Ok(())
}

fn tick_mp_regen(
    entity_id: u32,
    entity: &EntityState,
    combatant: &CombatantState,
    next_tick: u32,
    out: &mut Vec<GameEvent>,
) {
    if entity.current_hp <= 0 {
        return;
    }

    if !combatant.timed.is_active(TimedKind::MpRegenTick, next_tick) {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityMp {
            entity_id,
            delta: 1,
        }));
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id,
            kind: TimedKind::MpRegenTick,
            end_tick: next_tick.wrapping_add(MP_REGEN_INTERVAL),
        }));
    }
}

fn tick_combatant_effects(
    entity_id: u32,
    combatant: &CombatantState,
    next_tick: u32,
    out: &mut Vec<GameEvent>,
) {
    if combatant.timed.is_active(TimedKind::Poison, next_tick)
        && next_tick.is_multiple_of(STATUS_TICK_INTERVAL)
    {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityHp {
            entity_id,
            delta: -POISON_DAMAGE,
        }));
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
    let (leader_atk, _) = combat_attack_def(data, leader_entity)?;
    let current_tick = world.tick_counter;
    if leader_entity.current_hp <= 0
        || leader_combatant
            .timed
            .is_active(TimedKind::Stun, current_tick)
    {
        return Ok(());
    }

    if let Some((slot, skill)) = action.skill() {
        if leader_combatant
            .timed
            .is_active(TimedKind::SkillCooldown(slot as u8), current_tick)
        {
            return Ok(());
        }
        if leader_entity.current_mp < skill.mp_cost {
            return Ok(());
        }
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: TimedKind::SkillCooldown(slot as u8),
            end_tick: current_tick.wrapping_add(skill.cooldown),
        }));
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityMp {
            entity_id: leader_id,
            delta: -skill.mp_cost,
        }));
        resolve_skill_action(
            data,
            world,
            leader_id,
            leader_entity,
            leader_atk,
            skill,
            out,
        )?;
    } else {
        if leader_combatant
            .timed
            .is_active(TimedKind::AttackCooldown, current_tick)
        {
            return Ok(());
        }
        let (tx, ty) = leader_entity.facing.apply(leader_entity.x, leader_entity.y);
        let _ = damage_enemy_at_position(data, world, tx, ty, leader_atk, None, out)?;
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: TimedKind::AttackCooldown,
            end_tick: current_tick.wrapping_add(PLAYER_ATTACK_COOLDOWN),
        }));
    }
    Ok(())
}

fn resolve_skill_action(
    data: &GameData,
    world: &WorldState,
    leader_id: u32,
    leader: &EntityState,
    leader_atk: i32,
    skill: &Skill,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let base_damage = skill.power + leader_atk / 2;
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
            out.push(GameEvent::Entity(EntityEvent::ChangeEntityHp {
                entity_id: leader_id,
                delta: skill.heal_power.max(0),
            }));
        }
    }
    Ok(())
}

fn damage_enemy_at_position(
    _data: &GameData,
    world: &WorldState,
    x: usize,
    y: usize,
    raw_damage: i32,
    timed_effect: Option<(TimedKind, u32)>,
    out: &mut Vec<GameEvent>,
) -> Result<bool> {
    let current_tick = world.tick_counter;
    for enemy in &world.combat.enemies {
        let entity = world.entity(enemy.entity_id)?;
        if entity.x != x || entity.y != y {
            continue;
        }
        let effective_def = if enemy
            .combatant
            .timed
            .is_active(TimedKind::ArmorBreak, current_tick)
        {
            entity.stat.base_def / 2
        } else {
            entity.stat.base_def
        };
        let damage = raw_damage.saturating_sub(effective_def / 2).max(1);
        let next_hp = entity.current_hp.saturating_sub(damage).max(0);

        out.push(GameEvent::Entity(EntityEvent::ChangeEntityHp {
            entity_id: enemy.entity_id,
            delta: -damage,
        }));

        if next_hp <= 0 {
            return Ok(true);
        }

        if let Some((kind, duration)) = timed_effect
            && enemy.combatant.timed.time_left(kind, current_tick) < duration
        {
            out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: enemy.entity_id,
                kind,
                end_tick: current_tick.wrapping_add(duration),
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

    let leader_entity = world.leader_entity()?;
    let map = data.find_map(&leader_entity.map_id)?;

    let mut enemy_templates: Vec<&Enemy> = Vec::with_capacity(map.encounters.len());
    for (enemy_id, _) in &map.encounters {
        enemy_templates.push(data.find_enemy(enemy_id)?);
    }
    if enemy_templates.is_empty() || map.peaceful {
        out.push(GameEvent::Combat(CombatEvent::ClearEnemies));
        out.push(GameEvent::Combat(CombatEvent::SetActive(!map.peaceful)));
        out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
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
            out.push(GameEvent::Entity(EntityEvent::SetEntityCurrentHp {
                entity_id,
                value: enemy_data.hp,
            }));
            out.push(GameEvent::Entity(EntityEvent::SetEntityCurrentMp {
                entity_id,
                value: 0,
            }));
            out.push(GameEvent::Entity(EntityEvent::ClearEntityInventory {
                entity_id,
            }));
        }
    }

    out.push(GameEvent::Combat(CombatEvent::SetActive(true)));
    out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
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

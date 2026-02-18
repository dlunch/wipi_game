use alloc::vec::Vec;

use crate::game::state::{CombatStatsSnapshot, EntityId, EntityState, TimedEffect};
use crate::game::{CombatEvent, EntityEvent, GameEvent, LoadoutSlot};

pub fn emit_combat_stats(
    entity_id: EntityId,
    stats: &CombatStatsSnapshot,
    out: &mut Vec<GameEvent>,
) {
    let default_stats = CombatStatsSnapshot::default();
    out.push(GameEvent::Combat(CombatEvent::SetCombatantMaxHp {
        entity_id,
        max_hp: stats.max_hp,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantMaxMp {
        entity_id,
        max_mp: stats.max_mp,
    }));
    let base_hp = default_stats.current_hp.min(stats.max_hp).max(0);
    let hp_delta = stats.current_hp - base_hp;
    if hp_delta != 0 {
        out.push(GameEvent::Combat(CombatEvent::ChangeCombatantHp {
            entity_id,
            delta: hp_delta,
        }));
    }
    let base_mp = default_stats.current_mp.min(stats.max_mp).max(0);
    let mp_delta = stats.current_mp - base_mp;
    if mp_delta != 0 {
        out.push(GameEvent::Combat(CombatEvent::ChangeCombatantMp {
            entity_id,
            delta: mp_delta,
        }));
    }
    out.push(GameEvent::Combat(CombatEvent::SetCombatantAtk {
        entity_id,
        atk: stats.atk,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantDef {
        entity_id,
        def: stats.def,
    }));
}

pub fn emit_timed_effects(entity_id: EntityId, effects: &[TimedEffect], out: &mut Vec<GameEvent>) {
    for effect in effects {
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id,
            kind: effect.kind,
            end_tick: effect.end_tick,
        }));
    }
}

pub fn emit_entity_snapshot(entity: &EntityState, out: &mut Vec<GameEvent>) {
    out.push(GameEvent::Entity(EntityEvent::CreateEntity {
        entity_id: entity.id,
        kind: entity.kind,
        name: entity.name.clone(),
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityTransform {
        entity_id: entity.id,
        map_id: Some(entity.map_id.clone()),
        position: Some((entity.x, entity.y)),
        facing: Some(entity.facing),
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityLevel {
        entity_id: entity.id,
        level: entity.stat.level,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityExp {
        entity_id: entity.id,
        exp: entity.stat.exp,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityExpToNext {
        entity_id: entity.id,
        exp_to_next: entity.stat.exp_to_next,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityBaseMaxHp {
        entity_id: entity.id,
        base_max_hp: entity.stat.base_max_hp,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityBaseMaxMp {
        entity_id: entity.id,
        base_max_mp: entity.stat.base_max_mp,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityBaseAtk {
        entity_id: entity.id,
        base_atk: entity.stat.base_atk,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityBaseDef {
        entity_id: entity.id,
        base_def: entity.stat.base_def,
    }));
    out.push(GameEvent::Entity(EntityEvent::ClearEntityInventory {
        entity_id: entity.id,
    }));
    for stack in &entity.inventory {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
            entity_id: entity.id,
            item_id: stack.item_id.clone(),
            delta: stack.amount,
        }));
    }
    out.push(GameEvent::Entity(EntityEvent::SetEntityLoadoutSlot {
        entity_id: entity.id,
        slot: LoadoutSlot::Weapon,
        index: entity.loadout.weapon,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityLoadoutSlot {
        entity_id: entity.id,
        slot: LoadoutSlot::Armor,
        index: entity.loadout.armor,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityLoadoutSlot {
        entity_id: entity.id,
        slot: LoadoutSlot::Accessory,
        index: entity.loadout.accessory,
    }));
}

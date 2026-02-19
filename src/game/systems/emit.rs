use alloc::vec::Vec;

use crate::game::{
    game_event::{CombatEvent, EntityEvent, GameEvent, LoadoutSlot},
    state::{EntityId, EntityState, TimedEffect},
};

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
        source_enemy_id: None,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityTransform {
        entity_id: entity.id,
        map_id: Some(entity.map_id),
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
    out.push(GameEvent::Entity(EntityEvent::SetEntityCurrentHp {
        entity_id: entity.id,
        value: entity.current_hp,
    }));
    out.push(GameEvent::Entity(EntityEvent::SetEntityCurrentMp {
        entity_id: entity.id,
        value: entity.current_mp,
    }));
    out.push(GameEvent::Entity(EntityEvent::ClearEntityInventory {
        entity_id: entity.id,
    }));
    for stack in &entity.inventory {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
            entity_id: entity.id,
            item_id: stack.item_id,
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

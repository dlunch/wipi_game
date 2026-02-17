use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::WorldState;
use crate::data::Direction;
use crate::game::state::{
    AllyCombatantState, CombatState, CombatStatsSnapshot, CombatantState, EnemyCombatantState,
    EntityKind, EntityState, ItemStack, LoadoutState, PartyState, TimedEffect, TimedKind,
    TimedState,
};

const SAVE_VERSION: u32 = 2;

pub fn serialize(world: &WorldState) -> String {
    let mut lines = vec![
        format_args_to_string(&["VERSION", &SAVE_VERSION.to_string()]),
        format_args_to_string(&["WORLD_MAP", &world.occupancy.map_id]),
        format_args_to_string(&[
            "PARTY",
            &world.party.leader_id.to_string(),
            &join_u32(&world.party.companion_ids),
        ]),
        format_args_to_string(&[
            "COMBAT",
            if world.combat.active { "1" } else { "0" },
            &world.combat.update_counter.to_string(),
            &world.combat.respawn_timer.to_string(),
        ]),
    ];

    for entity in &world.entities.list {
        lines.push(format_args_to_string(&[
            "ENTITY",
            &entity.id.to_string(),
            entity_kind_code(entity.kind),
            &entity.name,
            &entity.map_id,
            &entity.x.to_string(),
            &entity.y.to_string(),
            direction_code(entity.facing),
            &entity.stat.level.to_string(),
            &entity.stat.exp.to_string(),
            &entity.stat.exp_to_next.to_string(),
            &entity.stat.base_max_hp.to_string(),
            &entity.stat.base_max_mp.to_string(),
            &entity.stat.base_atk.to_string(),
            &entity.stat.base_def.to_string(),
        ]));
        lines.push(format_args_to_string(&[
            "LOADOUT",
            &entity.id.to_string(),
            &opt_usize(entity.loadout.weapon),
            &opt_usize(entity.loadout.armor),
            &opt_usize(entity.loadout.accessory),
        ]));
        for stack in &entity.inventory {
            lines.push(format_args_to_string(&[
                "ITEM",
                &entity.id.to_string(),
                &stack.item_id,
                &stack.amount.to_string(),
            ]));
        }
    }

    for ally in &world.combat.allies {
        push_combatant_line("ALLY", ally.entity_id, &ally.combatant, &mut lines);
    }
    for enemy in &world.combat.enemies {
        lines.push(format_args_to_string(&[
            "ENEMY",
            &enemy.entity_id.to_string(),
            &enemy.source_enemy_id,
        ]));
        push_combatant_line("ENEMY_STATS", enemy.entity_id, &enemy.combatant, &mut lines);
    }

    for quest in &world.quests {
        lines.push(format_args_to_string(&[
            "QUEST",
            &quest.quest_id,
            &quest.current_count.to_string(),
            if quest.completed { "1" } else { "0" },
            if quest.rewarded { "1" } else { "0" },
        ]));
    }

    for (map_id, x, y) in &world.opened_treasures {
        lines.push(format_args_to_string(&[
            "TREASURE",
            map_id,
            &x.to_string(),
            &y.to_string(),
        ]));
    }

    let mut result = String::new();
    for line in lines {
        result.push_str(&line);
        result.push('\n');
    }
    result
}

fn push_combatant_line(
    prefix: &str,
    entity_id: u32,
    combatant: &CombatantState,
    lines: &mut Vec<String>,
) {
    let stats = combatant.stats;
    lines.push(format_args_to_string(&[
        prefix,
        &entity_id.to_string(),
        &stats.max_hp.to_string(),
        &stats.current_hp.to_string(),
        &stats.max_mp.to_string(),
        &stats.current_mp.to_string(),
        &stats.atk.to_string(),
        &stats.def.to_string(),
    ]));
    for effect in &combatant.timed.effects {
        lines.push(format_args_to_string(&[
            "TIMED",
            &entity_id.to_string(),
            &timed_kind_code(effect.kind),
            &effect.time_left.to_string(),
        ]));
    }
}

pub fn deserialize(data: &str, world: &mut WorldState) -> bool {
    if data.trim().is_empty() {
        return false;
    }
    let mut version = 0u32;
    let mut parsed_world_map = String::new();
    let mut parsed_party = PartyState::default();
    let mut parsed_entities: Vec<EntityState> = Vec::new();
    let mut parsed_allies: Vec<AllyCombatantState> = Vec::new();
    let mut parsed_enemies: Vec<EnemyCombatantState> = Vec::new();
    let mut parsed_timed: Vec<(u32, TimedEffect)> = Vec::new();
    let mut parsed_quests = Vec::new();
    let mut parsed_treasures = Vec::new();
    let mut parsed_combat_active = false;
    let mut parsed_update_counter = 0u32;
    let mut parsed_respawn_timer = 0u32;

    for line in data.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "VERSION" if parts.len() >= 2 => {
                version = parts[1].parse().unwrap_or(0);
            }
            "WORLD_MAP" if parts.len() >= 2 => parsed_world_map = parts[1].into(),
            "PARTY" if parts.len() >= 3 => {
                parsed_party.leader_id = parts[1].parse().unwrap_or(0);
                parsed_party.companion_ids = parse_u32_list(parts[2]);
            }
            "COMBAT" if parts.len() >= 4 => {
                parsed_combat_active = parts[1] == "1";
                parsed_update_counter = parts[2].parse().unwrap_or(0);
                parsed_respawn_timer = parts[3].parse().unwrap_or(0);
            }
            "ENTITY" if parts.len() >= 15 => {
                parsed_entities.push(EntityState {
                    id: parts[1].parse().unwrap_or(0),
                    kind: parse_entity_kind(parts[2]),
                    name: parts[3].into(),
                    map_id: parts[4].into(),
                    x: parts[5].parse().unwrap_or(0),
                    y: parts[6].parse().unwrap_or(0),
                    facing: parse_direction(parts[7]),
                    stat: crate::game::EntityStat {
                        level: parts[8].parse().unwrap_or(1).max(1),
                        exp: parts[9].parse().unwrap_or(0).max(0),
                        exp_to_next: parts[10].parse().unwrap_or(100).max(1),
                        base_max_hp: parts[11].parse().unwrap_or(80).max(1),
                        base_max_mp: parts[12].parse().unwrap_or(30).max(0),
                        base_atk: parts[13].parse().unwrap_or(12).max(0),
                        base_def: parts[14].parse().unwrap_or(8).max(0),
                    },
                    inventory: Vec::new(),
                    loadout: LoadoutState::default(),
                });
            }
            "LOADOUT" if parts.len() >= 5 => {
                let entity_id = parts[1].parse().unwrap_or(0);
                if let Some(entity) = parsed_entities
                    .iter_mut()
                    .find(|entity| entity.id == entity_id)
                {
                    entity.loadout = LoadoutState {
                        weapon: parse_opt_usize(parts[2]),
                        armor: parse_opt_usize(parts[3]),
                        accessory: parse_opt_usize(parts[4]),
                    };
                }
            }
            "ITEM" if parts.len() >= 4 => {
                let entity_id = parts[1].parse().unwrap_or(0);
                if let Some(entity) = parsed_entities
                    .iter_mut()
                    .find(|entity| entity.id == entity_id)
                {
                    entity.inventory.push(ItemStack {
                        item_id: parts[2].into(),
                        amount: parts[3].parse().unwrap_or(0).max(0),
                    });
                }
            }
            "ALLY" if parts.len() >= 8 => {
                let entity_id = parts[1].parse().unwrap_or(0);
                parsed_allies.push(AllyCombatantState {
                    entity_id,
                    combatant: CombatantState {
                        stats: CombatStatsSnapshot {
                            max_hp: parts[2].parse().unwrap_or(80).max(1),
                            current_hp: parts[3].parse().unwrap_or(80).max(0),
                            max_mp: parts[4].parse().unwrap_or(30).max(0),
                            current_mp: parts[5].parse().unwrap_or(30).max(0),
                            atk: parts[6].parse().unwrap_or(12).max(0),
                            def: parts[7].parse().unwrap_or(8).max(0),
                        },
                        timed: TimedState::default(),
                    },
                });
            }
            "ENEMY" if parts.len() >= 3 => {
                parsed_enemies.push(EnemyCombatantState {
                    entity_id: parts[1].parse().unwrap_or(0),
                    source_enemy_id: parts[2].into(),
                    combatant: CombatantState::default(),
                });
            }
            "ENEMY_STATS" if parts.len() >= 8 => {
                let entity_id = parts[1].parse().unwrap_or(0);
                if let Some(enemy) = parsed_enemies
                    .iter_mut()
                    .find(|enemy| enemy.entity_id == entity_id)
                {
                    enemy.combatant.stats = CombatStatsSnapshot {
                        max_hp: parts[2].parse().unwrap_or(1).max(1),
                        current_hp: parts[3].parse().unwrap_or(0).max(0),
                        max_mp: parts[4].parse().unwrap_or(0).max(0),
                        current_mp: parts[5].parse().unwrap_or(0).max(0),
                        atk: parts[6].parse().unwrap_or(0).max(0),
                        def: parts[7].parse().unwrap_or(0).max(0),
                    };
                }
            }
            "TIMED" if parts.len() >= 4 => {
                let entity_id = parts[1].parse().unwrap_or(0);
                parsed_timed.push((
                    entity_id,
                    TimedEffect {
                        kind: parse_timed_kind(parts[2]),
                        time_left: parts[3].parse().unwrap_or(0),
                    },
                ));
            }
            "QUEST" if parts.len() >= 5 => {
                parsed_quests.push(crate::data::QuestProgress {
                    quest_id: parts[1].into(),
                    current_count: parts[2].parse().unwrap_or(0),
                    completed: parts[3] == "1",
                    rewarded: parts[4] == "1",
                });
            }
            "TREASURE" if parts.len() >= 4 => {
                parsed_treasures.push((
                    parts[1].into(),
                    parts[2].parse().unwrap_or(0),
                    parts[3].parse().unwrap_or(0),
                ));
            }
            _ => {}
        }
    }

    if version != SAVE_VERSION {
        return false;
    }

    for (entity_id, effect) in parsed_timed {
        if let Some(ally) = parsed_allies
            .iter_mut()
            .find(|ally| ally.entity_id == entity_id)
        {
            ally.combatant.timed.effects.push(effect);
            continue;
        }
        if let Some(enemy) = parsed_enemies
            .iter_mut()
            .find(|enemy| enemy.entity_id == entity_id)
        {
            enemy.combatant.timed.effects.push(effect);
        }
    }

    let next_entity_id = parsed_entities
        .iter()
        .map(|entity| entity.id)
        .max()
        .unwrap_or(0)
        .wrapping_add(1)
        .max(1);

    *world = WorldState {
        entities: crate::game::EntityStore {
            list: parsed_entities,
            next_entity_id,
        },
        party: parsed_party,
        movement: Default::default(),
        combat: CombatState {
            active: parsed_combat_active,
            allies: parsed_allies,
            enemies: parsed_enemies,
            update_counter: parsed_update_counter,
            respawn_timer: parsed_respawn_timer,
        },
        quests: parsed_quests,
        opened_treasures: parsed_treasures,
        occupancy: crate::game::world::OccupancyState {
            map_id: parsed_world_map,
            width: 0,
            height: 0,
            npc_tiles: Vec::new(),
            enemy_tiles: Vec::new(),
        },
    };

    true
}

fn format_args_to_string(parts: &[&str]) -> String {
    let mut s = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            s.push(':');
        }
        s.push_str(part);
    }
    s
}

fn entity_kind_code(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Player => "P",
        EntityKind::Companion => "C",
        EntityKind::Enemy => "E",
        EntityKind::Npc => "N",
    }
}

fn parse_entity_kind(code: &str) -> EntityKind {
    match code {
        "P" => EntityKind::Player,
        "C" => EntityKind::Companion,
        "E" => EntityKind::Enemy,
        "N" => EntityKind::Npc,
        _ => EntityKind::Player,
    }
}

fn direction_code(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => "U",
        Direction::Down => "D",
        Direction::Left => "L",
        Direction::Right => "R",
    }
}

fn parse_direction(code: &str) -> Direction {
    match code {
        "U" => Direction::Up,
        "D" => Direction::Down,
        "L" => Direction::Left,
        "R" => Direction::Right,
        _ => Direction::Down,
    }
}

fn timed_kind_code(kind: TimedKind) -> String {
    match kind {
        TimedKind::Poison => "POISON".into(),
        TimedKind::Stun => "STUN".into(),
        TimedKind::ArmorBreak => "BREAK".into(),
        TimedKind::AttackCooldown => "ATK_CD".into(),
        TimedKind::SkillCooldown(slot) => format_args_to_string(&["SKILL", &slot.to_string()]),
        TimedKind::MpRegenTick => "MP_REGEN".into(),
    }
}

fn parse_timed_kind(code: &str) -> TimedKind {
    if let Some(slot) = code
        .strip_prefix("SKILL:")
        .and_then(|value| value.parse::<u8>().ok())
    {
        return TimedKind::SkillCooldown(slot);
    }
    match code {
        "POISON" => TimedKind::Poison,
        "STUN" => TimedKind::Stun,
        "BREAK" => TimedKind::ArmorBreak,
        "ATK_CD" => TimedKind::AttackCooldown,
        "MP_REGEN" => TimedKind::MpRegenTick,
        _ => TimedKind::Poison,
    }
}

fn opt_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("-1"))
}

fn parse_opt_usize(value: &str) -> Option<usize> {
    value
        .parse::<i32>()
        .ok()
        .filter(|value| *value >= 0)
        .map(|value| value as usize)
}

fn join_u32(values: &[u32]) -> String {
    let mut out = String::new();
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out
}

fn parse_u32_list(raw: &str) -> Vec<u32> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(',')
        .filter_map(|value| value.parse::<u32>().ok())
        .collect()
}

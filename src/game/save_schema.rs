use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::str::FromStr;

use anyhow::{Result, anyhow, ensure};

use crate::{
    data::{Direction, QuestProgress},
    game::{
        state::{
            AllyCombatantState, CombatState, CombatantState, EnemyCombatantState, EntityKind,
            EntityStat, EntityState, EntityStore, ItemStack, LoadoutState, PartyState, TimedEffect,
            TimedKind, TimedState,
        },
        world::{OccupancyState, WorldState},
    },
};

const SAVE_VERSION: u32 = 2;

pub fn serialize(world: &WorldState) -> String {
    let mut lines = vec![
        format_args_to_string(&["VERSION", &SAVE_VERSION.to_string()]),
        format_args_to_string(&["WORLD_MAP", &world.occupancy.map_id.to_string()]),
        format_args_to_string(&[
            "PARTY",
            &world.party.leader_id.to_string(),
            &join_u32(&world.party.companion_ids),
        ]),
        format_args_to_string(&[
            "COMBAT",
            if world.combat.active { "1" } else { "0" },
            &world.tick_counter.to_string(),
            &world.combat.respawn_timer.to_string(),
        ]),
    ];

    for entity in world.entities.iter() {
        lines.push(format_args_to_string(&[
            "ENTITY",
            &entity.id.to_string(),
            entity_kind_code(entity.kind),
            &entity.name,
            &entity.map_id.to_string(),
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
            &entity.current_hp.to_string(),
            &entity.current_mp.to_string(),
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
                &stack.item_id.to_string(),
                &stack.amount.to_string(),
            ]));
        }
    }

    for ally in &world.combat.allies {
        lines.push(format_args_to_string(&[
            "ALLY",
            &ally.entity_id.to_string(),
        ]));
        push_timed_lines(ally.entity_id, &ally.combatant, &mut lines);
    }
    for enemy in &world.combat.enemies {
        lines.push(format_args_to_string(&[
            "ENEMY",
            &enemy.entity_id.to_string(),
            &enemy.source_enemy_id.to_string(),
        ]));
        push_timed_lines(enemy.entity_id, &enemy.combatant, &mut lines);
    }

    for quest in &world.quests {
        lines.push(format_args_to_string(&[
            "QUEST",
            &quest.quest_id.to_string(),
            &quest.current_count.to_string(),
            if quest.completed { "1" } else { "0" },
            if quest.rewarded { "1" } else { "0" },
        ]));
    }

    for (map_id, x, y) in &world.opened_treasures {
        lines.push(format_args_to_string(&[
            "TREASURE",
            &map_id.to_string(),
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

fn push_timed_lines(entity_id: u32, combatant: &CombatantState, lines: &mut Vec<String>) {
    for effect in &combatant.timed.effects {
        lines.push(format_args_to_string(&[
            "TIMED",
            &entity_id.to_string(),
            &timed_kind_code(effect.kind),
            &effect.end_tick.to_string(),
        ]));
    }
}

pub fn deserialize(data: &str, world: &mut WorldState) -> Result<()> {
    ensure!(!data.trim().is_empty(), "empty save data");

    let mut version = 0u32;
    let mut parsed_world_map = 0u32;
    let mut parsed_party = PartyState::default();
    let mut parsed_entities = Vec::new();
    let mut parsed_allies = Vec::new();
    let mut parsed_enemies = Vec::new();
    let mut parsed_timed = Vec::new();
    let mut parsed_quests = Vec::new();
    let mut parsed_treasures = Vec::new();
    let mut parsed_combat_active = false;
    let mut parsed_tick_counter = 0u32;
    let mut parsed_respawn_timer = 0u32;

    for (line_no, line) in data
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
    {
        let parts = line.split(':').collect::<Vec<_>>();
        ensure!(
            !parts.is_empty(),
            "invalid save line {}: empty record",
            line_no + 1
        );

        match parts[0] {
            "VERSION" => {
                ensure!(parts.len() >= 2, "VERSION line is malformed");
                version = parse_value(parts[1], "VERSION")?;
            }
            "WORLD_MAP" => {
                ensure!(parts.len() >= 2, "WORLD_MAP line is malformed");
                parsed_world_map = parse_value(parts[1], "WORLD_MAP.map_id")?;
            }
            "PARTY" => {
                ensure!(parts.len() >= 3, "PARTY line is malformed");
                parsed_party.leader_id = parse_value(parts[1], "PARTY.leader_id")?;
                parsed_party.companion_ids = parse_u32_list(parts[2])?;
            }
            "COMBAT" => {
                ensure!(parts.len() >= 4, "COMBAT line is malformed");
                parsed_combat_active = parse_flag(parts[1], "COMBAT.active")?;
                parsed_tick_counter = parse_value(parts[2], "COMBAT.tick_counter")?;
                parsed_respawn_timer = parse_value(parts[3], "COMBAT.respawn_timer")?;
            }
            "ENTITY" => {
                ensure!(parts.len() >= 17, "ENTITY line is malformed");
                parsed_entities.push(EntityState {
                    id: parse_value(parts[1], "ENTITY.id")?,
                    kind: parse_entity_kind(parts[2])?,
                    name: parts[3].into(),
                    map_id: parse_value(parts[4], "ENTITY.map_id")?,
                    x: parse_value(parts[5], "ENTITY.x")?,
                    y: parse_value(parts[6], "ENTITY.y")?,
                    facing: parse_direction(parts[7])?,
                    stat: EntityStat {
                        level: parse_value::<i32>(parts[8], "ENTITY.level")?.max(1),
                        exp: parse_value::<i32>(parts[9], "ENTITY.exp")?.max(0),
                        exp_to_next: parse_value::<i32>(parts[10], "ENTITY.exp_to_next")?.max(1),
                        base_max_hp: parse_value::<i32>(parts[11], "ENTITY.base_max_hp")?.max(1),
                        base_max_mp: parse_value::<i32>(parts[12], "ENTITY.base_max_mp")?.max(0),
                        base_atk: parse_value::<i32>(parts[13], "ENTITY.base_atk")?.max(0),
                        base_def: parse_value::<i32>(parts[14], "ENTITY.base_def")?.max(0),
                    },
                    current_hp: parse_value::<i32>(parts[15], "ENTITY.current_hp")?.max(0),
                    current_mp: parse_value::<i32>(parts[16], "ENTITY.current_mp")?.max(0),
                    inventory: Vec::new(),
                    loadout: LoadoutState::default(),
                });
            }
            "LOADOUT" => {
                ensure!(parts.len() >= 5, "LOADOUT line is malformed");
                let entity_id = parse_value(parts[1], "LOADOUT.entity_id")?;
                let entity = parsed_entities
                    .iter_mut()
                    .find(|entity| entity.id == entity_id)
                    .ok_or_else(|| anyhow!("LOADOUT target entity not found: {}", entity_id))?;
                entity.loadout = LoadoutState {
                    weapon: parse_opt_usize(parts[2])?,
                    armor: parse_opt_usize(parts[3])?,
                    accessory: parse_opt_usize(parts[4])?,
                };
            }
            "ITEM" => {
                ensure!(parts.len() >= 4, "ITEM line is malformed");
                let entity_id = parse_value(parts[1], "ITEM.entity_id")?;
                let entity = parsed_entities
                    .iter_mut()
                    .find(|entity| entity.id == entity_id)
                    .ok_or_else(|| anyhow!("ITEM target entity not found: {}", entity_id))?;
                entity.inventory.push(ItemStack {
                    item_id: parse_value(parts[2], "ITEM.item_id")?,
                    amount: parse_value::<i32>(parts[3], "ITEM.amount")?.max(0),
                });
            }
            "ALLY" => {
                ensure!(parts.len() >= 2, "ALLY line is malformed");
                let entity_id = parse_value(parts[1], "ALLY.entity_id")?;
                parsed_allies.push(AllyCombatantState {
                    entity_id,
                    combatant: CombatantState {
                        timed: TimedState::default(),
                    },
                });
            }
            "ENEMY" => {
                ensure!(parts.len() >= 3, "ENEMY line is malformed");
                parsed_enemies.push(EnemyCombatantState {
                    entity_id: parse_value(parts[1], "ENEMY.entity_id")?,
                    source_enemy_id: parse_value(parts[2], "ENEMY.source_enemy_id")?,
                    combatant: CombatantState::default(),
                });
            }
            "TIMED" => {
                ensure!(parts.len() >= 4, "TIMED line is malformed");
                let entity_id = parse_value(parts[1], "TIMED.entity_id")?;
                parsed_timed.push((
                    entity_id,
                    TimedEffect {
                        kind: parse_timed_kind(parts[2])?,
                        end_tick: parse_value(parts[3], "TIMED.end_tick")?,
                    },
                ));
            }
            "QUEST" => {
                ensure!(parts.len() >= 5, "QUEST line is malformed");
                parsed_quests.push(QuestProgress {
                    quest_id: parse_value(parts[1], "QUEST.quest_id")?,
                    current_count: parse_value(parts[2], "QUEST.current_count")?,
                    completed: parse_flag(parts[3], "QUEST.completed")?,
                    rewarded: parse_flag(parts[4], "QUEST.rewarded")?,
                });
            }
            "TREASURE" => {
                ensure!(parts.len() >= 4, "TREASURE line is malformed");
                parsed_treasures.push((
                    parse_value(parts[1], "TREASURE.map_id")?,
                    parse_value(parts[2], "TREASURE.x")?,
                    parse_value(parts[3], "TREASURE.y")?,
                ));
            }
            record => return Err(anyhow!("unknown save record type: {}", record)),
        }
    }

    ensure!(
        version == SAVE_VERSION,
        "unsupported save version: {}",
        version
    );

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
            continue;
        }
        return Err(anyhow!(
            "TIMED target combatant not found for entity_id={}",
            entity_id
        ));
    }

    let next_entity_id = parsed_entities
        .iter()
        .fold(0u32, |acc, entity| acc.max(entity.id))
        .wrapping_add(1)
        .max(1);

    *world = WorldState {
        tick_counter: parsed_tick_counter,
        entities: EntityStore::from_list(parsed_entities, next_entity_id),
        party: parsed_party,
        movement: Default::default(),
        combat: CombatState {
            active: parsed_combat_active,
            allies: parsed_allies,
            enemies: parsed_enemies,
            respawn_timer: parsed_respawn_timer,
        },
        quests: parsed_quests,
        opened_treasures: parsed_treasures,
        occupancy: OccupancyState {
            map_id: parsed_world_map,
            width: 0,
            height: 0,
            npc_tiles: Vec::new(),
            enemy_tiles: Vec::new(),
            enemy_tile_counts: Vec::new(),
        },
    };

    Ok(())
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

fn parse_entity_kind(code: &str) -> Result<EntityKind> {
    match code {
        "P" => Ok(EntityKind::Player),
        "C" => Ok(EntityKind::Companion),
        "E" => Ok(EntityKind::Enemy),
        "N" => Ok(EntityKind::Npc),
        _ => Err(anyhow!("invalid ENTITY kind code: {}", code)),
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

fn parse_direction(code: &str) -> Result<Direction> {
    match code {
        "U" => Ok(Direction::Up),
        "D" => Ok(Direction::Down),
        "L" => Ok(Direction::Left),
        "R" => Ok(Direction::Right),
        _ => Err(anyhow!("invalid facing direction code: {}", code)),
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

fn parse_timed_kind(code: &str) -> Result<TimedKind> {
    if let Some(raw_slot) = code.strip_prefix("SKILL:") {
        let slot = parse_value(raw_slot, "TIMED.skill_slot")?;
        return Ok(TimedKind::SkillCooldown(slot));
    }
    match code {
        "POISON" => Ok(TimedKind::Poison),
        "STUN" => Ok(TimedKind::Stun),
        "BREAK" => Ok(TimedKind::ArmorBreak),
        "ATK_CD" => Ok(TimedKind::AttackCooldown),
        "MP_REGEN" => Ok(TimedKind::MpRegenTick),
        _ => Err(anyhow!("invalid timed kind code: {}", code)),
    }
}

fn opt_usize(value: Option<usize>) -> String {
    if let Some(value) = value {
        value.to_string()
    } else {
        String::from("-1")
    }
}

fn parse_opt_usize(value: &str) -> Result<Option<usize>> {
    let v = parse_value::<i32>(value, "LOADOUT.index")?;
    if v < 0 {
        return Ok(None);
    }
    Ok(Some(v as usize))
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

fn parse_u32_list(raw: &str) -> Result<Vec<u32>> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for value in raw.split(',') {
        out.push(parse_value(value, "PARTY.companion_id")?);
    }
    Ok(out)
}

fn parse_flag(raw: &str, field: &str) -> Result<bool> {
    match raw {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(anyhow!("invalid {} flag value: {}", field, raw)),
    }
}

fn parse_value<T>(raw: &str, field: &str) -> Result<T>
where
    T: FromStr,
{
    raw.parse::<T>()
        .map_err(|_| anyhow!("invalid {} value: {}", field, raw))
}

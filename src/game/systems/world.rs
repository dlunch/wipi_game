use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{DialogAction, QuestProgress, QuestType};
use crate::game::state::{GOLD_ITEM_ID, TimedKind};
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{
    CombatEvent, GameData, GameEvent, GameEventKind, MovementEvent, TileEvent, TransitionEvent,
    WorldEvent, WorldState,
};

struct WorldLogicResolver;

static WORLD_LOGIC_RESOLVER: WorldLogicResolver = WorldLogicResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&WORLD_LOGIC_RESOLVER]
}

impl DomainEventResolver for WorldLogicResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[
            GameEventKind::Movement,
            GameEventKind::ApplyDialogAction,
            GameEventKind::Combat,
            GameEventKind::RevivePlayer,
        ]
    }

    fn resolve(
        &self,
        data: &Rc<GameData>,
        world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let world = world.ok_or_else(|| anyhow!("No active world"))?;
        match event {
            GameEvent::Movement(MovementEvent::Tick(movement, Some(tile_event))) => {
                resolve_tile_event(data, world, movement.step, tile_event, out);
            }
            GameEvent::ApplyDialogAction(DialogAction::GiveQuest(id)) => {
                resolve_give_quest(world, id, out);
            }
            GameEvent::ApplyDialogAction(DialogAction::CompleteQuest(id)) => {
                resolve_complete_quest(data, world, id, out);
            }
            GameEvent::Combat(CombatEvent::RecoverMp { entity_id, amount }) => {
                resolve_recover_mp(world, *entity_id, *amount, out);
            }
            GameEvent::Combat(CombatEvent::Heal { entity_id, amount }) => {
                resolve_heal(world, *entity_id, *amount, out)
            }
            GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id,
                exp,
                gold,
            }) => resolve_kill_reward(data, world, enemy_id, *exp, *gold, out),
            GameEvent::Combat(CombatEvent::TakeDamage { entity_id, amount }) => {
                resolve_take_damage(data, world, *entity_id, *amount, out);
            }
            GameEvent::RevivePlayer => {
                resolve_revive_player(data, world, out);
            }
            _ => {}
        }
        Ok(())
    }
}

fn leader_id(world: &WorldState) -> Option<u32> {
    world.leader_id()
}

fn resolve_tile_event(
    data: &GameData,
    world: &WorldState,
    step: Option<(i32, i32)>,
    tile_event: &TileEvent,
    out: &mut Vec<GameEvent>,
) {
    let Some(leader) = world.leader_entity() else {
        return;
    };

    let (next_x, next_y) = if let Some((dx, dy)) = step {
        (
            (leader.x as i32 + dx).max(0) as usize,
            (leader.y as i32 + dy).max(0) as usize,
        )
    } else {
        (leader.x, leader.y)
    };

    match tile_event {
        TileEvent::Treasure => {
            let map_id = leader.map_id.clone();
            if world.is_treasure_opened(&map_id, next_x, next_y) {
                return;
            }
            out.push(GameEvent::World(WorldEvent::AddOpenedTreasure {
                map_id,
                x: next_x,
                y: next_y,
            }));
            if let Some(item_id) = data.newgame.treasure_item.as_deref()
                && let Some(leader_id) = leader_id(world)
            {
                out.push(GameEvent::World(WorldEvent::AddEntityItem {
                    entity_id: leader_id,
                    item_id: item_id.into(),
                    amount: 1,
                }));
            }
        }
        TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
            if target.is_empty() {
                return;
            }
            let Some(map) = data.find_map(target) else {
                return;
            };
            let Some(leader_id) = leader_id(world) else {
                return;
            };
            let (x, y) = map.find_player_start().unwrap_or((next_x, next_y));
            out.push(GameEvent::World(WorldEvent::SetWorldMap(map.id.clone())));
            out.push(GameEvent::World(WorldEvent::SetEntityMap {
                entity_id: leader_id,
                map_id: map.id.clone(),
            }));
            out.push(GameEvent::World(WorldEvent::SetEntityPosition {
                entity_id: leader_id,
                x,
                y,
            }));
            out.push(GameEvent::Transition(TransitionEvent::MapChanged));
        }
    }
}

fn resolve_give_quest(world: &WorldState, id: &str, out: &mut Vec<GameEvent>) {
    if world.quests.iter().any(|quest| quest.quest_id == id) {
        return;
    }
    out.push(GameEvent::World(WorldEvent::AddQuestProgress(
        QuestProgress {
            quest_id: id.into(),
            current_count: 0,
            completed: false,
            rewarded: false,
        },
    )));
}

fn resolve_complete_quest(data: &GameData, world: &WorldState, id: &str, out: &mut Vec<GameEvent>) {
    let can_reward = world
        .quests
        .iter()
        .any(|quest| quest.quest_id == id && quest.completed && !quest.rewarded);
    if !can_reward {
        return;
    }

    let Some(quest) = data.find_quest(id) else {
        return;
    };
    let Some(leader_id) = leader_id(world) else {
        return;
    };
    let Some(entity) = world.entity(leader_id) else {
        return;
    };
    let mut next_stat = entity.stat;
    next_stat.add_exp(quest.reward_exp);

    out.push(GameEvent::World(WorldEvent::SetEntityStat {
        entity_id: leader_id,
        stat: next_stat,
    }));
    out.push(GameEvent::World(WorldEvent::AddEntityItem {
        entity_id: leader_id,
        item_id: GOLD_ITEM_ID.into(),
        amount: quest.reward_gold.max(0),
    }));
    if let Some(item_id) = &quest.reward_item {
        out.push(GameEvent::World(WorldEvent::AddEntityItem {
            entity_id: leader_id,
            item_id: item_id.clone(),
            amount: 1,
        }));
    }

    if let Some(mut progress) = world.quests.iter().find(|q| q.quest_id == id).cloned() {
        progress.rewarded = true;
        out.push(GameEvent::World(WorldEvent::AddQuestProgress(progress)));
    }
}

fn resolve_recover_mp(world: &WorldState, entity_id: u32, amount: i32, out: &mut Vec<GameEvent>) {
    let Some(combatant) = world.combat.combatant(entity_id) else {
        return;
    };
    let mut stats = combatant.stats;
    if amount > 0 {
        stats.current_mp = (stats.current_mp + amount).min(stats.max_mp);
    } else if amount < 0 {
        stats.current_mp = (stats.current_mp + amount).max(0);
    }
    out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
        entity_id,
        stats,
    }));
}

fn resolve_heal(world: &WorldState, entity_id: u32, amount: i32, out: &mut Vec<GameEvent>) {
    if amount <= 0 {
        return;
    }
    let Some(combatant) = world.combat.combatant(entity_id) else {
        return;
    };
    let mut stats = combatant.stats;
    stats.current_hp = (stats.current_hp + amount).min(stats.max_hp);
    out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
        entity_id,
        stats,
    }));
}

fn resolve_kill_reward(
    data: &GameData,
    world: &WorldState,
    enemy_id: &str,
    exp: i32,
    gold: i32,
    out: &mut Vec<GameEvent>,
) {
    let Some(leader_id) = leader_id(world) else {
        return;
    };
    let Some(entity) = world.entity(leader_id) else {
        return;
    };

    let mut next_stat = entity.stat;
    next_stat.add_exp(exp);
    out.push(GameEvent::World(WorldEvent::SetEntityStat {
        entity_id: leader_id,
        stat: next_stat,
    }));
    out.push(GameEvent::World(WorldEvent::AddEntityItem {
        entity_id: leader_id,
        item_id: GOLD_ITEM_ID.into(),
        amount: gold.max(0),
    }));

    for progress in &world.quests {
        if progress.completed || progress.rewarded {
            continue;
        }
        if let Some(quest) = data.find_quest(&progress.quest_id)
            && quest.quest_type == QuestType::Kill
            && quest.target_id == enemy_id
        {
            let mut next = progress.clone();
            next.current_count = (next.current_count + 1).min(quest.target_count);
            if next.current_count >= quest.target_count {
                next.completed = true;
            }
            out.push(GameEvent::World(WorldEvent::AddQuestProgress(next)));
        }
    }
}

fn resolve_take_damage(
    data: &GameData,
    world: &WorldState,
    entity_id: u32,
    amount: i32,
    out: &mut Vec<GameEvent>,
) {
    if amount <= 0 {
        return;
    }
    let Some(combatant) = world.combat.combatant(entity_id) else {
        return;
    };
    let mut stats = combatant.stats;
    stats.current_hp = (stats.current_hp - amount).max(0);
    out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
        entity_id,
        stats,
    }));
    if stats.current_hp <= 0 {
        if Some(entity_id) == world.leader_id() {
            out.push(GameEvent::Transition(TransitionEvent::ToDead));
        } else if let Some(enemy) = world
            .combat
            .enemies
            .iter()
            .find(|e| e.entity_id == entity_id)
        {
            out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(entity_id)));
            if let Some(enemy_data) = data.find_enemy(&enemy.source_enemy_id) {
                out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                    enemy_id: enemy_data.id.clone(),
                    exp: enemy_data.exp,
                    gold: enemy_data.gold,
                }));
            }
        }
    }
}

fn resolve_revive_player(data: &GameData, world: &WorldState, out: &mut Vec<GameEvent>) {
    let Some(leader_id) = world.leader_id() else {
        return;
    };
    let Some(combatant) = world.combat.combatant(leader_id) else {
        return;
    };
    if combatant.stats.current_hp > 0 {
        return;
    }
    let current_gold = world.gold_amount(leader_id);
    let gold_penalty = (current_gold / 10).max(10);
    out.push(GameEvent::World(WorldEvent::RemoveEntityItem {
        entity_id: leader_id,
        item_id: GOLD_ITEM_ID.into(),
        amount: gold_penalty,
    }));

    let mut revived_stats = combatant.stats;
    revived_stats.current_hp = (revived_stats.max_hp / 2).max(1);
    revived_stats.current_mp = (revived_stats.max_mp / 2).max(0);
    out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
        entity_id: leader_id,
        stats: revived_stats,
    }));

    let village_map_id = data.newgame.start_map.clone();
    out.push(GameEvent::World(WorldEvent::SetWorldMap(
        village_map_id.clone(),
    )));
    out.push(GameEvent::World(WorldEvent::SetEntityMap {
        entity_id: leader_id,
        map_id: village_map_id.clone(),
    }));
    if let Some(village_map) = data.find_map(&village_map_id) {
        let (x, y) = village_map.find_player_start().unwrap_or((0, 0));
        out.push(GameEvent::World(WorldEvent::SetEntityPosition {
            entity_id: leader_id,
            x,
            y,
        }));
    }
    out.push(GameEvent::World(WorldEvent::ResetMovement));
    out.push(GameEvent::World(WorldEvent::ResetCombat));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
        entity_id: leader_id,
        kind: TimedKind::Poison,
        time_left: 0,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
        entity_id: leader_id,
        kind: TimedKind::Stun,
        time_left: 0,
    }));
    out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
        entity_id: leader_id,
        kind: TimedKind::ArmorBreak,
        time_left: 0,
    }));
    out.push(GameEvent::Transition(TransitionEvent::MapChanged));
    out.push(GameEvent::Transition(TransitionEvent::ToExplore));
}

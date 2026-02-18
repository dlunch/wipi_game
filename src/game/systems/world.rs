use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{DialogAction, QuestType};
use crate::game::state::{GOLD_ITEM_ID, TimedKind};
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{
    CombatEvent, EntityEvent, GameData, GameEvent, GameEventKind, MovementEvent, TileEvent,
    TransitionEvent, WorldEvent, WorldState,
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
                resolve_tile_event(data, world, movement.step, tile_event, out)?;
            }
            GameEvent::ApplyDialogAction(DialogAction::GiveQuest(id)) => {
                resolve_give_quest(world, id, out);
            }
            GameEvent::ApplyDialogAction(DialogAction::CompleteQuest(id)) => {
                resolve_complete_quest(data, world, id, out)?;
            }
            GameEvent::Combat(CombatEvent::ChangeCombatantHp { entity_id, delta }) => {
                resolve_combatant_hp_change(data, world, *entity_id, *delta, out)?;
            }
            GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id,
                exp,
                gold,
            }) => resolve_kill_reward(data, world, enemy_id, *exp, *gold, out)?,
            GameEvent::RevivePlayer => {
                resolve_revive_player(data, world, out)?;
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_tile_event(
    data: &GameData,
    world: &WorldState,
    step: Option<(i32, i32)>,
    tile_event: &TileEvent,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader = world.leader_entity()?;

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
                return Ok(());
            }
            out.push(GameEvent::World(WorldEvent::AddOpenedTreasure {
                map_id,
                x: next_x,
                y: next_y,
            }));
            if let Some(item_id) = data.newgame.treasure_item.as_deref() {
                let leader_id = world.leader_id()?;
                let _ = data.find_item(item_id)?;
                out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
                    entity_id: leader_id,
                    item_id: item_id.into(),
                    delta: 1,
                }));
            }
        }
        TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
            if target.is_empty() {
                return Ok(());
            }
            let map = data.find_map(target)?;
            let leader_id = world.leader_id()?;
            let (x, y) = map.find_player_start()?;
            out.push(GameEvent::World(WorldEvent::SetWorldMap(map.id.clone())));
            out.push(GameEvent::Entity(EntityEvent::SetEntityTransform {
                entity_id: leader_id,
                map_id: Some(map.id.clone()),
                position: Some((x, y)),
                facing: None,
            }));
            out.push(GameEvent::Transition(TransitionEvent::MapChanged));
        }
    }
    Ok(())
}

fn resolve_give_quest(world: &WorldState, id: &str, out: &mut Vec<GameEvent>) {
    if world.quests.iter().any(|quest| quest.quest_id == id) {
        return;
    }
    out.push(GameEvent::World(WorldEvent::CreateQuestProgress {
        quest_id: id.into(),
    }));
}

fn resolve_complete_quest(
    data: &GameData,
    world: &WorldState,
    id: &str,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let can_reward = world
        .quests
        .iter()
        .any(|quest| quest.quest_id == id && quest.completed && !quest.rewarded);
    if !can_reward {
        return Ok(());
    }

    let quest = data.find_quest(id)?;
    let leader_id = world.leader_id()?;
    out.push(GameEvent::Entity(EntityEvent::AddEntityExp {
        entity_id: leader_id,
        amount: quest.reward_exp,
    }));
    out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
        entity_id: leader_id,
        item_id: GOLD_ITEM_ID.into(),
        delta: quest.reward_gold.max(0),
    }));
    if let Some(item_id) = &quest.reward_item {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
            entity_id: leader_id,
            item_id: item_id.clone(),
            delta: 1,
        }));
    }

    out.push(GameEvent::World(WorldEvent::SetQuestRewarded {
        quest_id: id.into(),
        rewarded: true,
    }));
    Ok(())
}

fn resolve_kill_reward(
    data: &GameData,
    world: &WorldState,
    enemy_id: &str,
    exp: i32,
    gold: i32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader_id = world.leader_id()?;
    out.push(GameEvent::Entity(EntityEvent::AddEntityExp {
        entity_id: leader_id,
        amount: exp,
    }));
    out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
        entity_id: leader_id,
        item_id: GOLD_ITEM_ID.into(),
        delta: gold.max(0),
    }));

    for progress in &world.quests {
        if progress.completed || progress.rewarded {
            continue;
        }
        let quest = data.find_quest(&progress.quest_id)?;
        if quest.quest_type == QuestType::Kill && quest.target_id == enemy_id {
            out.push(GameEvent::World(WorldEvent::ChangeQuestCurrentCount {
                quest_id: progress.quest_id.clone(),
                delta: 1,
            }));
        }
    }
    Ok(())
}

fn resolve_combatant_hp_change(
    data: &GameData,
    world: &WorldState,
    entity_id: u32,
    delta: i32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    if delta >= 0 {
        return Ok(());
    }
    let combatant = world.combat.combatant(entity_id)?;
    let next_hp = (combatant.stats.current_hp + delta).max(0);
    if next_hp > 0 || combatant.stats.current_hp <= 0 {
        return Ok(());
    }

    if next_hp <= 0 {
        if world.leader_id()? == entity_id {
            out.push(GameEvent::Transition(TransitionEvent::ToDead));
        } else if let Some(enemy) = world
            .combat
            .enemies
            .iter()
            .find(|e| e.entity_id == entity_id)
        {
            out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(entity_id)));
            let enemy_data = data.find_enemy(&enemy.source_enemy_id)?;
            out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id: enemy_data.id.clone(),
                exp: enemy_data.exp,
                gold: enemy_data.gold,
            }));
        }
    }
    Ok(())
}

fn resolve_revive_player(
    data: &GameData,
    world: &WorldState,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader_id = world.leader_id()?;
    let combatant = world.combat.combatant(leader_id)?;

    if combatant.stats.current_hp > 0 {
        return Ok(());
    }
    let current_gold = world.gold_amount(leader_id)?;
    let gold_penalty = (current_gold / 10).max(10);
    out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
        entity_id: leader_id,
        item_id: GOLD_ITEM_ID.into(),
        delta: -gold_penalty,
    }));

    let target_hp = (combatant.stats.max_hp / 2).max(1);
    let target_mp = (combatant.stats.max_mp / 2).max(0);
    let hp_delta = target_hp - combatant.stats.current_hp;
    let mp_delta = target_mp - combatant.stats.current_mp;
    if hp_delta != 0 {
        out.push(GameEvent::Combat(CombatEvent::ChangeCombatantHp {
            entity_id: leader_id,
            delta: hp_delta,
        }));
    }
    if mp_delta != 0 {
        out.push(GameEvent::Combat(CombatEvent::ChangeCombatantMp {
            entity_id: leader_id,
            delta: mp_delta,
        }));
    }

    let village_map_id = data.newgame.start_map.clone();
    out.push(GameEvent::World(WorldEvent::SetWorldMap(
        village_map_id.clone(),
    )));
    let village_position = Some(data.find_map(&village_map_id)?.find_player_start()?);
    out.push(GameEvent::Entity(EntityEvent::SetEntityTransform {
        entity_id: leader_id,
        map_id: Some(village_map_id),
        position: village_position,
        facing: None,
    }));
    out.push(GameEvent::Movement(MovementEvent::ClearPressedDirections));
    out.push(GameEvent::Movement(MovementEvent::SetMoveCooldown(0)));
    out.push(GameEvent::Combat(CombatEvent::ClearEnemies));
    out.push(GameEvent::Combat(CombatEvent::SetActive(false)));
    out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(0)));
    out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
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
    out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
        entity_id: leader_id,
        kind: TimedKind::AttackCooldown,
        time_left: 0,
    }));
    for slot in 0..3u8 {
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: TimedKind::SkillCooldown(slot),
            time_left: 0,
        }));
    }
    out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
        entity_id: leader_id,
        kind: TimedKind::MpRegenTick,
        time_left: 0,
    }));
    out.push(GameEvent::Transition(TransitionEvent::MapChanged));
    out.push(GameEvent::Transition(TransitionEvent::ToExplore));
    Ok(())
}

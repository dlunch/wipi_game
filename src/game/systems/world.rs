use alloc::{rc::Rc, vec, vec::Vec};

use anyhow::{Result, anyhow};

use super::resolver::DomainEventResolver;
use crate::{
    data::{DialogAction, QuestType},
    game::{
        game_data::GameData,
        game_event::{
            CombatEvent, EntityEvent, GameEvent, GameEventKind, MovementEvent, TileEvent,
            TransitionEvent, WorldEvent,
        },
        state::{GOLD_ITEM_ID, TimedKind},
        world::WorldState,
    },
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
            GameEventKind::Entity,
            GameEventKind::Combat,
            GameEventKind::RevivePlayer,
            GameEventKind::OpenShopById,
            GameEventKind::ShopBuyItem,
            GameEventKind::ShopSellItem,
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
            GameEvent::Entity(EntityEvent::ChangeEntityHp { entity_id, delta }) => {
                resolve_entity_hp_change(data, world, *entity_id, *delta, out)?;
            }
            GameEvent::Entity(EntityEvent::SetEntityCurrentHp { entity_id, value }) => {
                let current_hp = world.entity(*entity_id)?.current_hp;
                resolve_entity_hp_change(data, world, *entity_id, *value - current_hp, out)?;
            }
            GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id,
                exp,
                gold,
            }) => resolve_kill_reward(data, world, enemy_id, *exp, *gold, out)?,
            GameEvent::OpenShopById(shop_id) => {
                resolve_open_shop(data, world, shop_id, out)?;
            }
            GameEvent::ShopBuyItem(item_data_id) => {
                resolve_shop_sell_cache_after_buy(data, world, *item_data_id, out)?;
            }
            GameEvent::ShopSellItem(item_data_id) => {
                resolve_shop_sell_cache_after_sell(data, world, *item_data_id, out)?;
            }
            GameEvent::RevivePlayer => {
                resolve_revive_player(data, world, out)?;
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_open_shop(
    data: &GameData,
    world: &WorldState,
    shop_id: &u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let shop = data.find_shop(*shop_id)?;
    let mut buy_item_ids = Vec::with_capacity(shop.items.len());
    for item_id in &shop.items {
        buy_item_ids.push(*item_id);
    }
    out.push(GameEvent::SetShopBuyItemIds(buy_item_ids));
    out.push(GameEvent::SetShopSellItemIds(sell_item_data_ids(world)?));
    Ok(())
}

fn resolve_shop_sell_cache_after_buy(
    data: &GameData,
    world: &WorldState,
    item_data_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader_id = world.leader_id()?;
    let item = data.find_item(item_data_id)?;
    if world.gold_amount(leader_id)? < item.price {
        return Ok(());
    }

    let mut sell_item_ids = sell_item_data_ids(world)?;
    sell_item_ids.push(item_data_id);
    out.push(GameEvent::SetShopSellItemIds(sell_item_ids));
    Ok(())
}

fn resolve_shop_sell_cache_after_sell(
    data: &GameData,
    world: &WorldState,
    item_data_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let item = data.find_item(item_data_id)?;
    if item.id == GOLD_ITEM_ID {
        return Ok(());
    }

    let leader = world.leader_entity()?;
    let has_item = leader
        .inventory
        .iter()
        .any(|stack| stack.item_id == item.id && stack.amount > 0);
    if !has_item {
        return Ok(());
    }

    let mut sell_item_ids = sell_item_data_ids(world)?;
    if let Some(index) = sell_item_ids
        .iter()
        .position(|current_item_data_id| *current_item_data_id == item_data_id)
    {
        sell_item_ids.remove(index);
    }
    out.push(GameEvent::SetShopSellItemIds(sell_item_ids));
    Ok(())
}

fn sell_item_data_ids(world: &WorldState) -> Result<Vec<u32>> {
    let leader = world.leader_entity()?;
    let mut sell_item_ids = Vec::new();
    for stack in &leader.inventory {
        if stack.item_id == GOLD_ITEM_ID || stack.amount <= 0 {
            continue;
        }
        for _ in 0..stack.amount.max(0) {
            sell_item_ids.push(stack.item_id);
        }
    }
    Ok(sell_item_ids)
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
            let map_id = leader.map_id;
            if world.is_treasure_opened(map_id, next_x, next_y) {
                return Ok(());
            }
            out.push(GameEvent::World(WorldEvent::AddOpenedTreasure {
                map_id,
                x: next_x,
                y: next_y,
            }));
            if let Some(item_id) = data.newgame_config().treasure_item {
                let leader_id = world.leader_id()?;
                data.find_item(item_id)?;
                out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
                    entity_id: leader_id,
                    item_id,
                    delta: 1,
                }));
            }
        }
        TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
            if *target == 0 {
                return Ok(());
            }
            let map = data.find_map(*target)?;
            let leader_id = world.leader_id()?;
            let (x, y) = map.find_player_start()?;
            out.push(GameEvent::World(WorldEvent::SetWorldMap(map.id)));
            out.push(GameEvent::Entity(EntityEvent::SetEntityTransform {
                entity_id: leader_id,
                map_id: Some(map.id),
                position: Some((x, y)),
                facing: None,
            }));
            out.push(GameEvent::Transition(TransitionEvent::MapChanged));
        }
    }
    Ok(())
}

fn resolve_give_quest(world: &WorldState, id: &u32, out: &mut Vec<GameEvent>) {
    if world.quests.iter().any(|quest| quest.quest_id == *id) {
        return;
    }
    out.push(GameEvent::World(WorldEvent::CreateQuestProgress {
        quest_id: *id,
    }));
}

fn resolve_complete_quest(
    data: &GameData,
    world: &WorldState,
    id: &u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let can_reward = world
        .quests
        .iter()
        .any(|quest| quest.quest_id == *id && quest.completed && !quest.rewarded);
    if !can_reward {
        return Ok(());
    }

    let quest = data.find_quest(*id)?;
    let leader_id = world.leader_id()?;
    out.push(GameEvent::Entity(EntityEvent::AddEntityExp {
        entity_id: leader_id,
        amount: quest.reward_exp,
    }));
    push_item_delta(out, leader_id, GOLD_ITEM_ID, quest.reward_gold.max(0));
    if let Some(item_id) = quest.reward_item {
        push_item_delta(out, leader_id, item_id, 1);
    }

    out.push(GameEvent::World(WorldEvent::SetQuestRewarded {
        quest_id: *id,
        rewarded: true,
    }));
    Ok(())
}

fn resolve_kill_reward(
    data: &GameData,
    world: &WorldState,
    enemy_id: &u32,
    exp: i32,
    gold: i32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader_id = world.leader_id()?;
    out.push(GameEvent::Entity(EntityEvent::AddEntityExp {
        entity_id: leader_id,
        amount: exp,
    }));
    push_item_delta(out, leader_id, GOLD_ITEM_ID, gold.max(0));

    for progress in &world.quests {
        if progress.completed || progress.rewarded {
            continue;
        }
        let quest = data.find_quest(progress.quest_id)?;
        if quest.quest_type == QuestType::Kill && quest.target_id == *enemy_id {
            out.push(GameEvent::World(WorldEvent::ChangeQuestCurrentCount {
                quest_id: progress.quest_id,
                delta: 1,
            }));
        }
    }
    Ok(())
}

fn resolve_entity_hp_change(
    data: &GameData,
    world: &WorldState,
    entity_id: u32,
    delta: i32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let entity = world.entity(entity_id)?;
    if delta >= 0 || entity.current_hp <= 0 {
        return Ok(());
    }
    if (entity.current_hp + delta).max(0) > 0 {
        return Ok(());
    }

    if world.leader_id()? == entity_id {
        out.push(GameEvent::Transition(TransitionEvent::ToDead));
    } else if let Some(enemy) = world
        .combat
        .enemies
        .iter()
        .find(|e| e.entity_id == entity_id)
    {
        out.push(GameEvent::Combat(CombatEvent::RemoveEnemy(entity_id)));
        let enemy_data = data.find_enemy(enemy.source_enemy_id)?;
        out.push(GameEvent::Combat(CombatEvent::GrantKillReward {
            enemy_id: enemy_data.id,
            exp: enemy_data.exp,
            gold: enemy_data.gold,
        }));
    }
    Ok(())
}

fn resolve_revive_player(
    data: &GameData,
    world: &WorldState,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader_id = world.leader_id()?;
    let leader = world.entity(leader_id)?;

    if leader.current_hp > 0 {
        return Ok(());
    }
    let current_gold = world.gold_amount(leader_id)?;
    let gold_penalty = (current_gold / 10).max(10);
    push_item_delta(out, leader_id, GOLD_ITEM_ID, -gold_penalty);

    let target_hp = (leader.stat.base_max_hp / 2).max(1);
    let target_mp = (leader.stat.base_max_mp / 2).max(0);
    let hp_delta = target_hp - leader.current_hp;
    let mp_delta = target_mp - leader.current_mp;
    if hp_delta != 0 {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityHp {
            entity_id: leader_id,
            delta: hp_delta,
        }));
    }
    if mp_delta != 0 {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityMp {
            entity_id: leader_id,
            delta: mp_delta,
        }));
    }

    let village_map_id = data.newgame_config().start_map;
    out.push(GameEvent::World(WorldEvent::SetWorldMap(village_map_id)));
    let village_position = Some(data.find_map(village_map_id)?.find_player_start()?);
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
    out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(0)));
    clear_combatant_timed_effects(out, leader_id);
    out.push(GameEvent::Transition(TransitionEvent::MapChanged));
    out.push(GameEvent::Transition(TransitionEvent::ToExplore));
    Ok(())
}

fn push_item_delta(out: &mut Vec<GameEvent>, entity_id: u32, item_id: u32, delta: i32) {
    out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
        entity_id,
        item_id,
        delta,
    }));
}

fn clear_combatant_timed_effects(out: &mut Vec<GameEvent>, entity_id: u32) {
    for kind in [
        TimedKind::Poison,
        TimedKind::Stun,
        TimedKind::ArmorBreak,
        TimedKind::AttackCooldown,
        TimedKind::MpRegenTick,
    ] {
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id,
            kind,
            end_tick: 0,
        }));
    }
    for slot in 0..3u8 {
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id,
            kind: TimedKind::SkillCooldown(slot),
            end_tick: 0,
        }));
    }
}

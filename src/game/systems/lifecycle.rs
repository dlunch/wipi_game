use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::save::load_game;
use crate::game::state::{
    CombatStatsSnapshot, EntityId, EntityStat, EntityState, GOLD_ITEM_ID, ItemStack, TimedEffect,
    TimedKind, combat_attack_def,
};
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{
    CombatEvent, DialogState, EntityEvent, GameData, GameEvent, GameEventKind, LoadoutSlot,
    MovementEvent, TransitionEvent, WorldEvent, WorldState,
};

pub enum LoadingEvent {
    Tick,
    Advance(usize),
    Loaded,
}

#[derive(Clone, Copy)]
pub enum LifecycleEvent {
    ResetUi,
    ContinueSetup,
    SetMenuHasSaveData(bool),
}

struct LifecycleResolver;

static LIFECYCLE_RESOLVER: LifecycleResolver = LifecycleResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&LIFECYCLE_RESOLVER]
}

impl DomainEventResolver for LifecycleResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[
            GameEventKind::StartNewGame,
            GameEventKind::ContinueGame,
            GameEventKind::Lifecycle,
        ]
    }

    fn resolve(
        &self,
        data: &Rc<GameData>,
        _world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::StartNewGame => {
                out.push(GameEvent::Lifecycle(LifecycleEvent::ResetUi));
                Self::setup_new_game_events(data, out)?;
                out.push(GameEvent::Transition(TransitionEvent::ToExplore));
                if let Some(dialog_state) = Self::intro_dialog_state(data)? {
                    out.push(GameEvent::OpenDialogState(dialog_state));
                }
            }
            GameEvent::ContinueGame => {
                out.push(GameEvent::Lifecycle(LifecycleEvent::ResetUi));
                out.push(GameEvent::Lifecycle(LifecycleEvent::ContinueSetup));
            }
            GameEvent::Lifecycle(LifecycleEvent::ContinueSetup) => {
                Self::setup_continue_events(data, out)?;
                out.push(GameEvent::Transition(TransitionEvent::ToExplore));
            }
            _ => {}
        }

        Ok(())
    }
}

impl LifecycleResolver {
    fn intro_dialog_state(data: &GameData) -> Result<Option<DialogState>> {
        let Some((dialog_id, npc_name)) = data.newgame.intro_dialog.as_ref() else {
            return Ok(None);
        };
        let dialog = data.find_dialog(dialog_id)?;
        Ok(Some(DialogState::from_dialog(npc_name.clone(), dialog)))
    }

    fn setup_new_game_events(data: &GameData, out: &mut Vec<GameEvent>) -> Result<()> {
        let config = &data.newgame;
        let leader_id: EntityId = 1;

        let mut leader = EntityState::new_player(
            leader_id,
            config.player_name.clone(),
            config.start_map.clone(),
        );
        leader.stat = EntityStat::default();
        add_or_inc_stack(&mut leader.inventory, GOLD_ITEM_ID, 50);

        if let Some(weapon_id) = config.equip_weapon.as_deref() {
            let idx = push_stack(&mut leader.inventory, weapon_id, 1);
            leader.loadout.weapon = Some(idx);
        }
        if let Some(armor_id) = config.equip_armor.as_deref() {
            let idx = push_stack(&mut leader.inventory, armor_id, 1);
            leader.loadout.armor = Some(idx);
        }
        for start_item in &config.items {
            add_or_inc_stack(
                &mut leader.inventory,
                &start_item.item_id,
                start_item.count.max(0),
            );
        }

        let map = data.find_map(&config.start_map)?;
        let (x, y) = map.find_player_start()?;
        leader.map_id = map.id.clone();
        leader.x = x;
        leader.y = y;

        let leader_stats = snapshot_for_entity(data, &leader)?;

        out.push(GameEvent::World(WorldEvent::CreateWorld));
        out.push(GameEvent::World(WorldEvent::SetWorldMap(
            leader.map_id.clone(),
        )));
        out.push(GameEvent::Entity(EntityEvent::SetLeaderEntity(leader_id)));
        out.push(GameEvent::Entity(EntityEvent::ClearCompanionEntities));
        emit_entity_snapshot(&leader, out);
        out.push(GameEvent::Combat(CombatEvent::SetActive(true)));
        emit_combat_stats(leader_id, &leader_stats, out);
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: TimedKind::MpRegenTick,
            time_left: 0,
        }));
        out.push(GameEvent::Combat(CombatEvent::ClearEnemies));
        out.push(GameEvent::Movement(MovementEvent::ClearPressedDirections));
        out.push(GameEvent::Movement(MovementEvent::SetMoveCooldown(0)));
        out.push(GameEvent::Transition(TransitionEvent::MapChanged));
        Ok(())
    }

    fn setup_continue_events(data: &GameData, out: &mut Vec<GameEvent>) -> Result<()> {
        let mut world = WorldState::empty();
        load_game(&mut world)?;

        let leader = world.leader_entity()?;
        data.find_map(&leader.map_id)?;
        let leader_id = world.leader_id()?;

        emit_world_snapshot(&world, out);
        if world
            .combat
            .combatant(leader_id)?
            .timed
            .time_left(TimedKind::MpRegenTick)
            == 0
        {
            out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: leader_id,
                kind: TimedKind::MpRegenTick,
                time_left: 0,
            }));
        }
        out.push(GameEvent::Transition(TransitionEvent::MapChanged));
        Ok(())
    }
}

fn snapshot_for_entity(data: &GameData, entity: &EntityState) -> Result<CombatStatsSnapshot> {
    let (atk, def) = combat_attack_def(data, entity)?;

    Ok(CombatStatsSnapshot {
        max_hp: entity.stat.base_max_hp,
        current_hp: entity.stat.base_max_hp,
        max_mp: entity.stat.base_max_mp,
        current_mp: entity.stat.base_max_mp,
        atk,
        def,
    })
}

fn push_stack(inventory: &mut Vec<ItemStack>, item_id: &str, amount: i32) -> usize {
    inventory.push(ItemStack::new(item_id, amount));
    inventory.len() - 1
}

fn add_or_inc_stack(inventory: &mut Vec<ItemStack>, item_id: &str, amount: i32) {
    if amount <= 0 {
        return;
    }
    if let Some(stack) = inventory.iter_mut().find(|stack| stack.item_id == item_id) {
        stack.amount += amount;
    } else {
        inventory.push(ItemStack::new(item_id, amount));
    }
}

fn emit_world_snapshot(world: &WorldState, out: &mut Vec<GameEvent>) {
    out.push(GameEvent::World(WorldEvent::CreateWorld));
    out.push(GameEvent::World(WorldEvent::SetWorldMap(
        world.occupancy.map_id.clone(),
    )));
    out.push(GameEvent::Entity(EntityEvent::SetLeaderEntity(
        world.party.leader_id,
    )));
    out.push(GameEvent::Entity(EntityEvent::ClearCompanionEntities));
    for entity in world.entities.iter() {
        emit_entity_snapshot(entity, out);
    }
    for companion_id in &world.party.companion_ids {
        out.push(GameEvent::Entity(EntityEvent::AddCompanionEntity(
            *companion_id,
        )));
    }
    for quest in &world.quests {
        emit_quest_snapshot(
            &quest.quest_id,
            quest.current_count,
            quest.completed,
            quest.rewarded,
            out,
        );
    }
    for (map_id, x, y) in &world.opened_treasures {
        out.push(GameEvent::World(WorldEvent::AddOpenedTreasure {
            map_id: map_id.clone(),
            x: *x,
            y: *y,
        }));
    }
    out.push(GameEvent::Combat(CombatEvent::SetActive(
        world.combat.active,
    )));
    for ally in &world.combat.allies {
        emit_combat_stats(ally.entity_id, &ally.combatant.stats, out);
        emit_timed_effects(ally.entity_id, &ally.combatant.timed.effects, out);
    }
    for enemy in &world.combat.enemies {
        emit_combat_stats(enemy.entity_id, &enemy.combatant.stats, out);
        emit_timed_effects(enemy.entity_id, &enemy.combatant.timed.effects, out);
    }
    out.push(GameEvent::Combat(CombatEvent::SetUpdateCounter(
        world.combat.update_counter,
    )));
    out.push(GameEvent::Combat(CombatEvent::SetRespawnTimer(
        world.combat.respawn_timer,
    )));
    out.push(GameEvent::Movement(MovementEvent::ClearPressedDirections));
    out.push(GameEvent::Movement(MovementEvent::SetMoveCooldown(
        world.movement.move_cooldown,
    )));
}

fn emit_quest_snapshot(
    quest_id: &str,
    current_count: i32,
    completed: bool,
    rewarded: bool,
    out: &mut Vec<GameEvent>,
) {
    let quest_id = String::from(quest_id);
    out.push(GameEvent::World(WorldEvent::CreateQuestProgress {
        quest_id: quest_id.clone(),
    }));
    out.push(GameEvent::World(WorldEvent::ChangeQuestCurrentCount {
        quest_id: quest_id.clone(),
        delta: current_count,
    }));
    out.push(GameEvent::World(WorldEvent::SetQuestCompleted {
        quest_id: quest_id.clone(),
        completed,
    }));
    out.push(GameEvent::World(WorldEvent::SetQuestRewarded {
        quest_id,
        rewarded,
    }));
}

fn emit_timed_effects(entity_id: EntityId, effects: &[TimedEffect], out: &mut Vec<GameEvent>) {
    for effect in effects {
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id,
            kind: effect.kind,
            time_left: effect.time_left,
        }));
    }
}

fn emit_combat_stats(entity_id: EntityId, stats: &CombatStatsSnapshot, out: &mut Vec<GameEvent>) {
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

fn emit_entity_snapshot(entity: &EntityState, out: &mut Vec<GameEvent>) {
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

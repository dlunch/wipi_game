use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::game_data::GameData;
use crate::game::game_event::{
    CombatEvent, EntityEvent, GameEvent, GameEventKind, MovementEvent, TransitionEvent, WorldEvent,
};
use crate::game::save::load_game;
use crate::game::state::{EntityId, EntityStat, EntityState, GOLD_ITEM_ID, ItemStack, TimedKind};
use crate::game::ui::state::DialogState;
use crate::game::world::WorldState;

use super::emit::{emit_entity_snapshot, emit_timed_effects};
use super::resolver::DomainEventResolver;

pub enum LoadingEvent {
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

        leader.current_hp = leader.stat.base_max_hp;
        leader.current_mp = leader.stat.base_max_mp;

        out.push(GameEvent::World(WorldEvent::CreateWorld));
        out.push(GameEvent::World(WorldEvent::SetWorldMap(
            leader.map_id.clone(),
        )));
        out.push(GameEvent::Entity(EntityEvent::SetLeaderEntity(leader_id)));
        out.push(GameEvent::Entity(EntityEvent::ClearCompanionEntities));
        emit_entity_snapshot(&leader, out);
        out.push(GameEvent::Combat(CombatEvent::SetActive(true)));
        out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
            entity_id: leader_id,
            kind: TimedKind::MpRegenTick,
            end_tick: 0,
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
            .time_left(TimedKind::MpRegenTick, world.tick_counter)
            == 0
        {
            out.push(GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: leader_id,
                kind: TimedKind::MpRegenTick,
                end_tick: 0,
            }));
        }
        out.push(GameEvent::Transition(TransitionEvent::MapChanged));
        Ok(())
    }
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
        emit_timed_effects(ally.entity_id, &ally.combatant.timed.effects, out);
    }
    for enemy in &world.combat.enemies {
        emit_timed_effects(enemy.entity_id, &enemy.combatant.timed.effects, out);
    }
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

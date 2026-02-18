use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::Direction;
use crate::data::QuestProgress;
use crate::game::state::{
    AllyCombatantState, CombatState, CombatantState, EnemyCombatantState, EntityId, EntityKind,
    EntityState, EntityStore, GOLD_ITEM_ID, PartyState,
};
use crate::game::{
    CombatEvent, GameData, GameEvent, GameEventKind, GameEventSubscriber, LoadoutSlot,
    MovementState, WorldEvent,
};

#[derive(Debug, Clone, Default)]
pub struct OccupancyState {
    pub map_id: String,
    pub width: usize,
    pub height: usize,
    pub npc_tiles: Vec<bool>,
    pub enemy_tiles: Vec<bool>,
}

#[derive(Clone, Default)]
pub struct WorldState {
    pub entities: EntityStore,
    pub party: PartyState,
    pub movement: MovementState,
    pub combat: CombatState,
    pub quests: Vec<QuestProgress>,
    pub opened_treasures: Vec<(String, usize, usize)>,
    pub occupancy: OccupancyState,
}

impl WorldState {
    pub fn empty() -> Self {
        Self {
            entities: EntityStore {
                list: Vec::new(),
                next_entity_id: 1,
            },
            party: PartyState::default(),
            movement: MovementState::default(),
            combat: CombatState::default(),
            quests: Vec::new(),
            opened_treasures: Vec::new(),
            occupancy: OccupancyState::default(),
        }
    }

    pub fn leader_id(&self) -> Option<EntityId> {
        let leader_id = self.party.leader_id;
        (leader_id != 0).then_some(leader_id)
    }

    pub fn leader_entity(&self) -> Option<&EntityState> {
        self.leader_id()
            .and_then(|leader_id| self.entities.get(leader_id))
    }

    pub fn entity(&self, entity_id: EntityId) -> Option<&EntityState> {
        self.entities.get(entity_id)
    }

    pub fn entity_mut(&mut self, entity_id: EntityId) -> Option<&mut EntityState> {
        self.entities.get_mut(entity_id)
    }

    pub fn has_quest(&self, quest_id: &str) -> bool {
        self.quests
            .iter()
            .any(|q| q.quest_id == quest_id && !q.rewarded)
    }

    pub fn is_quest_complete(&self, quest_id: &str) -> bool {
        self.quests
            .iter()
            .any(|q| q.quest_id == quest_id && q.completed)
    }

    pub fn is_treasure_opened(&self, map_id: &str, x: usize, y: usize) -> bool {
        self.opened_treasures
            .iter()
            .any(|(m, tx, ty)| m == map_id && *tx == x && *ty == y)
    }

    pub fn has_item(&self, entity_id: EntityId, item_id: &str) -> bool {
        self.entity(entity_id).is_some_and(|entity| {
            entity
                .inventory
                .iter()
                .any(|stack| stack.item_id == item_id && stack.amount > 0)
        })
    }

    pub fn item_amount(&self, entity_id: EntityId, item_id: &str) -> i32 {
        self.entity(entity_id)
            .and_then(|entity| {
                entity
                    .inventory
                    .iter()
                    .find_map(|stack| (stack.item_id == item_id).then_some(stack.amount))
            })
            .unwrap_or(0)
            .max(0)
    }

    pub fn gold_amount(&self, entity_id: EntityId) -> i32 {
        self.item_amount(entity_id, GOLD_ITEM_ID)
    }

    pub fn add_item_amount(&mut self, entity_id: EntityId, item_id: &str, amount: i32) {
        if amount <= 0 {
            return;
        }
        let Some(entity) = self.entity_mut(entity_id) else {
            return;
        };

        if let Some(stack) = entity
            .inventory
            .iter_mut()
            .find(|stack| stack.item_id == item_id)
        {
            stack.amount += amount;
        } else {
            entity
                .inventory
                .push(crate::game::state::ItemStack::new(item_id, amount));
        }
    }

    pub fn remove_item_amount(&mut self, entity_id: EntityId, item_id: &str, amount: i32) {
        if amount <= 0 {
            return;
        }
        let Some(entity) = self.entity_mut(entity_id) else {
            return;
        };

        if let Some(index) = entity
            .inventory
            .iter()
            .position(|stack| stack.item_id == item_id)
        {
            let stack = &mut entity.inventory[index];
            stack.amount = (stack.amount - amount).max(0);
            if stack.amount <= 0 {
                entity.inventory.remove(index);
                fix_loadout_after_remove(&mut entity.loadout.weapon, index);
                fix_loadout_after_remove(&mut entity.loadout.armor, index);
                fix_loadout_after_remove(&mut entity.loadout.accessory, index);
            }
        }
    }

    pub fn is_occupied(&self, x: usize, y: usize) -> bool {
        if self.occupancy.width == 0 || self.occupancy.height == 0 {
            return false;
        }
        if x >= self.occupancy.width || y >= self.occupancy.height {
            return true;
        }
        let idx = y * self.occupancy.width + x;
        self.occupancy.npc_tiles.get(idx).copied().unwrap_or(false)
            || self
                .occupancy
                .enemy_tiles
                .get(idx)
                .copied()
                .unwrap_or(false)
    }

    pub(crate) fn is_occupied_on_map(&self, map: &crate::data::Map, x: usize, y: usize) -> bool {
        if x >= map.width || y >= map.height {
            return true;
        }
        if self.occupancy.map_id != map.id {
            return false;
        }
        self.is_occupied(x, y)
    }

    pub fn apply_event(&mut self, data: &GameData, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::World(world_event) => {
                self.apply_world_event(data, world_event);
            }
            GameEvent::Combat(combat_event) => {
                self.apply_combat_event(combat_event, event)?;
            }
            GameEvent::Movement(_) | GameEvent::Explore(_) | GameEvent::Transition(_) => {
                self.movement.apply_event(event)?;
                if let GameEvent::Movement(crate::game::MovementEvent::Tick(movement_event, _)) =
                    event
                    && let Some(leader_id) = self.leader_id()
                    && let Some(leader) = self.entities.get_mut(leader_id)
                {
                    if let Some((dx, dy)) = movement_event.facing {
                        leader.facing = match (dx, dy) {
                            (0, -1) => crate::data::Direction::Up,
                            (0, 1) => crate::data::Direction::Down,
                            (-1, 0) => crate::data::Direction::Left,
                            (1, 0) => crate::data::Direction::Right,
                            _ => leader.facing,
                        };
                    }
                    if let Some((dx, dy)) = movement_event.step {
                        leader.x = (leader.x as i32 + dx).max(0) as usize;
                        leader.y = (leader.y as i32 + dy).max(0) as usize;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_world_event(&mut self, data: &GameData, event: &WorldEvent) {
        match event {
            WorldEvent::CreateWorld => {
                *self = WorldState::empty();
            }
            WorldEvent::SetWorldMap(map_id) => {
                self.rebuild_npc_occupancy_for_map(data, map_id);
                self.rebuild_enemy_occupancy();
            }
            WorldEvent::SetLeaderEntity(entity_id) => {
                self.party.leader_id = *entity_id;
                self.sync_allies_with_party();
            }
            WorldEvent::ClearCompanionEntities => {
                self.party.companion_ids.clear();
                self.sync_allies_with_party();
            }
            WorldEvent::AddCompanionEntity(entity_id) => {
                if !self.party.companion_ids.contains(entity_id) {
                    self.party.companion_ids.push(*entity_id);
                }
                self.sync_allies_with_party();
            }
            WorldEvent::CreateEntity {
                entity_id,
                kind,
                name,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.kind = *kind;
                    entity.name = name.clone();
                } else {
                    self.entities.upsert(EntityState {
                        id: *entity_id,
                        kind: *kind,
                        name: name.clone(),
                        map_id: String::new(),
                        x: 0,
                        y: 0,
                        facing: Direction::Down,
                        stat: Default::default(),
                        inventory: Vec::new(),
                        loadout: Default::default(),
                    });
                }
                self.sync_combat_entry_for_entity(data, *entity_id);
                self.sync_allies_with_party();
                self.rebuild_enemy_occupancy();
            }
            WorldEvent::RemoveEntity(entity_id) => {
                self.entities.remove(*entity_id);
                self.party.companion_ids.retain(|id| *id != *entity_id);
                if self.party.leader_id == *entity_id {
                    self.party.leader_id = 0;
                }
                self.combat
                    .allies
                    .retain(|ally| ally.entity_id != *entity_id);
                self.combat
                    .enemies
                    .retain(|enemy| enemy.entity_id != *entity_id);
                self.rebuild_enemy_occupancy();
            }
            WorldEvent::SetEntityTransform {
                entity_id,
                map_id,
                position,
                facing,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    if let Some(map_id) = map_id {
                        entity.map_id = map_id.clone();
                    }
                    if let Some((x, y)) = position {
                        entity.x = *x;
                        entity.y = *y;
                    }
                    if let Some(facing) = facing {
                        entity.facing = *facing;
                    }
                }
                if map_id.is_some() || position.is_some() {
                    self.rebuild_enemy_occupancy();
                }
            }
            WorldEvent::SetEntityLevel { entity_id, level } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.level = *level;
                }
            }
            WorldEvent::SetEntityExp { entity_id, exp } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.exp = *exp;
                }
            }
            WorldEvent::SetEntityExpToNext {
                entity_id,
                exp_to_next,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.exp_to_next = *exp_to_next;
                }
            }
            WorldEvent::SetEntityBaseMaxHp {
                entity_id,
                base_max_hp,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.base_max_hp = *base_max_hp;
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::SetEntityBaseMaxMp {
                entity_id,
                base_max_mp,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.base_max_mp = *base_max_mp;
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::SetEntityBaseAtk {
                entity_id,
                base_atk,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.base_atk = *base_atk;
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::SetEntityBaseDef {
                entity_id,
                base_def,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.base_def = *base_def;
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::AddEntityExp { entity_id, amount } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat.add_exp(*amount);
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::ClearEntityInventory { entity_id } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.inventory.clear();
                    entity.loadout.weapon = None;
                    entity.loadout.armor = None;
                    entity.loadout.accessory = None;
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::SetEntityLoadoutSlot {
                entity_id,
                slot,
                index,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    match slot {
                        LoadoutSlot::Weapon => {
                            entity.loadout.weapon = *index;
                        }
                        LoadoutSlot::Armor => {
                            entity.loadout.armor = *index;
                        }
                        LoadoutSlot::Accessory => {
                            entity.loadout.accessory = *index;
                        }
                    }
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::ChangeEntityItem {
                entity_id,
                item_id,
                delta,
            } => {
                if *delta > 0 {
                    self.add_item_amount(*entity_id, item_id, *delta);
                } else if *delta < 0 {
                    self.remove_item_amount(*entity_id, item_id, -*delta);
                }
                self.sync_combat_stats_for_entity(data, *entity_id);
            }
            WorldEvent::CreateQuestProgress { quest_id } => {
                self.ensure_quest_progress(quest_id);
            }
            WorldEvent::ChangeQuestCurrentCount { quest_id, delta } => {
                let progress = self.ensure_quest_progress(quest_id);
                progress.current_count = (progress.current_count + *delta).max(0);
                if let Some(quest) = data.find_quest(quest_id) {
                    progress.current_count = progress.current_count.min(quest.target_count.max(0));
                    if progress.current_count >= quest.target_count {
                        progress.completed = true;
                    }
                }
            }
            WorldEvent::SetQuestCompleted {
                quest_id,
                completed,
            } => {
                self.ensure_quest_progress(quest_id).completed = *completed;
            }
            WorldEvent::SetQuestRewarded { quest_id, rewarded } => {
                self.ensure_quest_progress(quest_id).rewarded = *rewarded;
            }
            WorldEvent::AddOpenedTreasure { map_id, x, y } => {
                if !self.is_treasure_opened(map_id, *x, *y) {
                    self.opened_treasures.push((map_id.clone(), *x, *y));
                }
            }
        }
    }

    fn apply_combat_event(&mut self, event: &CombatEvent, game_event: &GameEvent) -> Result<()> {
        let enemy_was_alive = match event {
            CombatEvent::SetCombatantCurrentHp { entity_id, .. } => self
                .combat
                .enemies
                .iter()
                .find(|enemy| enemy.entity_id == *entity_id)
                .map(|enemy| enemy.combatant.stats.current_hp > 0),
            _ => None,
        };

        self.combat.apply_event(game_event)?;
        match event {
            CombatEvent::MoveEnemy { entity_id, x, y } => {
                if let Some(enemy_entity) = self.entities.get_mut(*entity_id) {
                    enemy_entity.x = *x;
                    enemy_entity.y = *y;
                }
                self.rebuild_enemy_occupancy();
            }
            CombatEvent::RemoveEnemy(entity_id) => {
                self.entities.remove(*entity_id);
                self.rebuild_enemy_occupancy();
            }
            CombatEvent::ClearEnemies => {
                self.rebuild_enemy_occupancy();
            }
            CombatEvent::SetCombatantCurrentHp { entity_id, .. } => {
                if let Some(was_alive) = enemy_was_alive {
                    let is_alive = self
                        .combat
                        .enemies
                        .iter()
                        .find(|enemy| enemy.entity_id == *entity_id)
                        .is_some_and(|enemy| enemy.combatant.stats.current_hp > 0);
                    if was_alive != is_alive {
                        self.rebuild_enemy_occupancy();
                    }
                } else if self
                    .combat
                    .enemies
                    .iter()
                    .any(|enemy| enemy.entity_id == *entity_id)
                {
                    self.rebuild_enemy_occupancy();
                }
            }
            CombatEvent::SetActive(_)
            | CombatEvent::ClearAllies
            | CombatEvent::SetCombatantMaxHp { .. }
            | CombatEvent::SetCombatantMaxMp { .. }
            | CombatEvent::SetCombatantCurrentMp { .. }
            | CombatEvent::SetCombatantAtk { .. }
            | CombatEvent::SetCombatantDef { .. }
            | CombatEvent::SetCombatantTimed { .. }
            | CombatEvent::SetUpdateCounter(_)
            | CombatEvent::SetRespawnTimer(_)
            | CombatEvent::GrantKillReward { .. }
            | CombatEvent::RecoverMp { .. }
            | CombatEvent::Heal { .. }
            | CombatEvent::TakeDamage { .. } => {}
        }
        Ok(())
    }

    fn clear_enemy_occupancy(&mut self) {
        for occupied in &mut self.occupancy.enemy_tiles {
            *occupied = false;
        }
    }

    fn rebuild_npc_occupancy_for_map(&mut self, data: &GameData, map_id: &str) {
        self.occupancy.map_id = map_id.into();
        if let Some(map) = data.find_map(map_id) {
            self.occupancy.width = map.width;
            self.occupancy.height = map.height;
            let len = map.width * map.height;
            self.occupancy.npc_tiles = vec![false; len];
            self.occupancy.enemy_tiles = vec![false; len];

            for (x, y, _) in &map.npcs {
                if *x < map.width && *y < map.height {
                    self.occupancy.npc_tiles[*y * map.width + *x] = true;
                }
            }
        } else {
            self.occupancy = OccupancyState::default();
        }
    }

    fn rebuild_enemy_occupancy(&mut self) {
        self.clear_enemy_occupancy();
        if self.occupancy.width == 0 || self.occupancy.height == 0 {
            return;
        }
        for enemy in &self.combat.enemies {
            if enemy.combatant.stats.current_hp <= 0 {
                continue;
            }
            let Some(entity) = self.entities.get(enemy.entity_id) else {
                continue;
            };
            if entity.map_id != self.occupancy.map_id {
                continue;
            }
            if entity.x < self.occupancy.width && entity.y < self.occupancy.height {
                self.occupancy.enemy_tiles[entity.y * self.occupancy.width + entity.x] = true;
            }
        }
    }

    fn sync_allies_with_party(&mut self) {
        let mut party_ids: Vec<EntityId> = Vec::with_capacity(1 + self.party.companion_ids.len());
        if self.party.leader_id != 0 {
            party_ids.push(self.party.leader_id);
        }
        party_ids.extend(self.party.companion_ids.iter().copied());

        self.combat
            .allies
            .retain(|ally| party_ids.contains(&ally.entity_id));

        for entity_id in party_ids {
            if self.entities.get(entity_id).is_some()
                && self
                    .combat
                    .allies
                    .iter()
                    .all(|ally| ally.entity_id != entity_id)
            {
                self.combat.allies.push(AllyCombatantState {
                    entity_id,
                    combatant: CombatantState::default(),
                });
            }
        }
    }

    fn sync_combat_entry_for_entity(&mut self, data: &GameData, entity_id: EntityId) {
        let Some(entity) = self.entities.get(entity_id) else {
            return;
        };

        if matches!(entity.kind, EntityKind::Enemy) {
            let source_enemy_id = if data.find_enemy(&entity.name).is_some() {
                entity.name.clone()
            } else {
                data.enemies
                    .iter()
                    .find(|enemy| enemy.name == entity.name)
                    .map(|enemy| enemy.id.clone())
                    .unwrap_or_else(|| entity.name.clone())
            };

            if let Some(enemy) = self
                .combat
                .enemies
                .iter_mut()
                .find(|enemy| enemy.entity_id == entity_id)
            {
                enemy.source_enemy_id = source_enemy_id;
            } else {
                self.combat.enemies.push(EnemyCombatantState {
                    entity_id,
                    source_enemy_id,
                    combatant: CombatantState::default(),
                });
            }
        }
    }

    fn sync_combat_stats_for_entity(&mut self, data: &GameData, entity_id: EntityId) {
        let Some(entity) = self.entities.get(entity_id) else {
            return;
        };
        let Some(combatant) = self.combat.combatant_mut(entity_id) else {
            return;
        };

        let max_hp = entity.stat.base_max_hp.max(0);
        let max_mp = entity.stat.base_max_mp.max(0);
        combatant.stats.max_hp = max_hp;
        combatant.stats.max_mp = max_mp;
        combatant.stats.current_hp = combatant.stats.current_hp.min(max_hp).max(0);
        combatant.stats.current_mp = combatant.stats.current_mp.min(max_mp).max(0);

        let mut atk = entity.stat.base_atk;
        let mut def = entity.stat.base_def;

        if let Some(index) = entity.loadout.weapon
            && let Some(stack) = entity.inventory.get(index)
            && let Some(item) = data.find_item(&stack.item_id)
        {
            atk += item.atk();
        }
        if let Some(index) = entity.loadout.armor
            && let Some(stack) = entity.inventory.get(index)
            && let Some(item) = data.find_item(&stack.item_id)
        {
            def += item.def();
        }
        if let Some(index) = entity.loadout.accessory
            && let Some(stack) = entity.inventory.get(index)
            && let Some(item) = data.find_item(&stack.item_id)
        {
            atk += item.atk();
            def += item.def();
        }

        combatant.stats.atk = atk;
        combatant.stats.def = def;
    }

    fn ensure_quest_progress(&mut self, quest_id: &str) -> &mut QuestProgress {
        if let Some(index) = self
            .quests
            .iter()
            .position(|quest| quest.quest_id == quest_id)
        {
            &mut self.quests[index]
        } else {
            self.quests.push(QuestProgress {
                quest_id: quest_id.into(),
                current_count: 0,
                completed: false,
                rewarded: false,
            });
            let index = self.quests.len() - 1;
            &mut self.quests[index]
        }
    }
}

fn fix_loadout_after_remove(equipped: &mut Option<usize>, removed_index: usize) {
    if let Some(index) = equipped {
        if *index > removed_index {
            *index -= 1;
        } else if *index == removed_index {
            *equipped = None;
        }
    }
}

impl GameEventSubscriber for WorldState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(
            kind,
            GameEventKind::World
                | GameEventKind::Combat
                | GameEventKind::Movement
                | GameEventKind::Explore
                | GameEventKind::Transition
        )
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use anyhow::{Result, anyhow};

    use crate::data::{Direction, Quest, QuestType};
    use crate::game::state::{
        CombatantState, EnemyCombatantState, EntityKind, EntityState, ItemStack,
    };
    use crate::game::{CombatEvent, GameData, GameEvent, WorldEvent};

    use super::{OccupancyState, WorldState};

    #[test]
    fn add_entity_exp_syncs_combat_snapshot() -> Result<()> {
        let data = GameData::default();
        let mut world = WorldState::empty();

        world.apply_event(&data, &GameEvent::World(WorldEvent::CreateWorld))?;
        world.apply_event(&data, &GameEvent::World(WorldEvent::SetLeaderEntity(1)))?;
        world.apply_event(
            &data,
            &GameEvent::World(WorldEvent::CreateEntity {
                entity_id: 1,
                kind: EntityKind::Player,
                name: String::from("Hero"),
            }),
        )?;
        world.apply_event(
            &data,
            &GameEvent::World(WorldEvent::AddEntityExp {
                entity_id: 1,
                amount: 100,
            }),
        )?;

        let entity = world
            .entity(1)
            .ok_or_else(|| anyhow!("entity should exist"))?;
        let combatant = world
            .combat
            .combatant(1)
            .ok_or_else(|| anyhow!("combatant should exist"))?;

        assert_eq!(entity.stat.level, 2);
        assert_eq!(combatant.stats.max_hp, entity.stat.base_max_hp);
        assert_eq!(combatant.stats.max_mp, entity.stat.base_max_mp);
        assert_eq!(combatant.stats.atk, entity.stat.base_atk);
        assert_eq!(combatant.stats.def, entity.stat.base_def);
        assert_eq!(combatant.stats.current_hp, 80);
        assert_eq!(combatant.stats.current_mp, 30);
        Ok(())
    }

    #[test]
    fn quest_progress_uses_delta_and_clamps_to_target() -> Result<()> {
        let mut data = GameData::default();
        data.quests.push(Quest {
            id: String::from("q-kill"),
            name: String::from("Quest"),
            description: String::from("Desc"),
            quest_type: QuestType::Kill,
            target_id: String::from("slime"),
            target_count: 3,
            reward_exp: 1,
            reward_gold: 1,
            reward_item: None,
        });

        let mut world = WorldState::empty();
        world.apply_event(
            &data,
            &GameEvent::World(WorldEvent::CreateQuestProgress {
                quest_id: String::from("q-kill"),
            }),
        )?;
        world.apply_event(
            &data,
            &GameEvent::World(WorldEvent::ChangeQuestCurrentCount {
                quest_id: String::from("q-kill"),
                delta: 1,
            }),
        )?;
        world.apply_event(
            &data,
            &GameEvent::World(WorldEvent::ChangeQuestCurrentCount {
                quest_id: String::from("q-kill"),
                delta: 1,
            }),
        )?;

        let progress = world
            .quests
            .iter()
            .find(|progress| progress.quest_id == "q-kill")
            .ok_or_else(|| anyhow!("quest progress should exist"))?;
        assert_eq!(progress.current_count, 2);
        assert!(!progress.completed);

        world.apply_event(
            &data,
            &GameEvent::World(WorldEvent::ChangeQuestCurrentCount {
                quest_id: String::from("q-kill"),
                delta: 10,
            }),
        )?;
        let progress = world
            .quests
            .iter()
            .find(|progress| progress.quest_id == "q-kill")
            .ok_or_else(|| anyhow!("quest progress should exist"))?;
        assert_eq!(progress.current_count, 3);
        assert!(progress.completed);
        Ok(())
    }

    #[test]
    fn enemy_occupancy_updates_only_on_alive_transition() -> Result<()> {
        let data = GameData::default();
        let mut world = WorldState::empty();

        world.occupancy = OccupancyState {
            map_id: String::from("map"),
            width: 3,
            height: 3,
            npc_tiles: vec![false; 9],
            enemy_tiles: vec![false; 9],
        };

        world.entities.upsert(EntityState {
            id: 10,
            kind: EntityKind::Enemy,
            name: String::from("slime"),
            map_id: String::from("map"),
            x: 1,
            y: 1,
            facing: Direction::Down,
            stat: Default::default(),
            inventory: Vec::<ItemStack>::new(),
            loadout: Default::default(),
        });
        world.combat.enemies.push(EnemyCombatantState {
            entity_id: 10,
            source_enemy_id: String::from("slime"),
            combatant: CombatantState::default(),
        });
        world.rebuild_enemy_occupancy();
        assert!(world.is_occupied(1, 1));

        world.apply_event(
            &data,
            &GameEvent::Combat(CombatEvent::SetCombatantCurrentHp {
                entity_id: 10,
                current_hp: 5,
            }),
        )?;
        assert!(world.is_occupied(1, 1));

        world.apply_event(
            &data,
            &GameEvent::Combat(CombatEvent::SetCombatantCurrentHp {
                entity_id: 10,
                current_hp: 0,
            }),
        )?;
        assert!(!world.is_occupied(1, 1));

        world.apply_event(
            &data,
            &GameEvent::Combat(CombatEvent::SetCombatantCurrentHp {
                entity_id: 10,
                current_hp: 3,
            }),
        )?;
        assert!(world.is_occupied(1, 1));
        Ok(())
    }
}

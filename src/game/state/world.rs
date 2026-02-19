use alloc::{collections::BTreeSet, string::String, vec, vec::Vec};

use anyhow::{Result, anyhow, ensure};

use crate::{
    data::{Direction, Map, QuestProgress},
    game::{
        game_data::GameData,
        game_event::{
            CombatEvent, EntityEvent, GameEvent, GameEventKind, GameEventSubscriber, LoadoutSlot,
            MovementEvent, WorldEvent,
        },
        state::{
            AllyCombatantState, CombatState, CombatantState, EnemyCombatantState, EntityId,
            EntityKind, EntityStat, EntityState, EntityStore, GOLD_ITEM_ID, ItemStack,
            MovementState, PartyState,
        },
    },
};

#[derive(Debug, Default)]
pub struct OccupancyState {
    pub map_id: String,
    pub width: usize,
    pub height: usize,
    pub npc_tiles: Vec<bool>,
    pub enemy_tiles: Vec<bool>,
    pub enemy_tile_counts: Vec<u16>,
}

#[derive(Default)]
pub struct WorldState {
    pub tick_counter: u32,
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
            tick_counter: 0,
            entities: EntityStore::new(),
            party: PartyState::default(),
            movement: MovementState::default(),
            combat: CombatState::default(),
            quests: Vec::new(),
            opened_treasures: Vec::new(),
            occupancy: OccupancyState::default(),
        }
    }

    pub fn leader_id(&self) -> Result<EntityId> {
        let leader_id = self.party.leader_id;
        if leader_id == 0 {
            return Err(anyhow!("Leader entity is not set"));
        }
        Ok(leader_id)
    }

    pub fn leader_entity(&self) -> Result<&EntityState> {
        let leader_id = self.leader_id()?;
        self.entities.get(leader_id)
    }

    pub fn entity(&self, entity_id: EntityId) -> Result<&EntityState> {
        self.entities.get(entity_id)
    }

    pub fn entity_mut(&mut self, entity_id: EntityId) -> Result<&mut EntityState> {
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

    pub fn has_item(&self, entity_id: EntityId, item_id: &str) -> Result<bool> {
        Ok(self.item_amount(entity_id, item_id)? > 0)
    }

    pub fn item_amount(&self, entity_id: EntityId, item_id: &str) -> Result<i32> {
        let entity = self.entity(entity_id)?;
        Ok(entity
            .inventory
            .iter()
            .find(|stack| stack.item_id == item_id)
            .map_or(0, |stack| stack.amount.max(0)))
    }

    pub fn gold_amount(&self, entity_id: EntityId) -> Result<i32> {
        self.item_amount(entity_id, GOLD_ITEM_ID)
    }

    pub fn add_item_amount(
        &mut self,
        entity_id: EntityId,
        item_id: &str,
        amount: i32,
    ) -> Result<()> {
        if amount <= 0 {
            return Ok(());
        }
        let entity = self.entity_mut(entity_id)?;

        if let Some(stack) = entity
            .inventory
            .iter_mut()
            .find(|stack| stack.item_id == item_id)
        {
            stack.amount += amount;
        } else {
            entity.inventory.push(ItemStack::new(item_id, amount));
        }
        Ok(())
    }

    pub fn remove_item_amount(
        &mut self,
        entity_id: EntityId,
        item_id: &str,
        amount: i32,
    ) -> Result<()> {
        if amount <= 0 {
            return Ok(());
        }
        let entity = self.entity_mut(entity_id)?;

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
        Ok(())
    }

    pub fn is_occupied(&self, x: usize, y: usize) -> bool {
        if self.occupancy.width == 0 || self.occupancy.height == 0 {
            return false;
        }
        if x >= self.occupancy.width || y >= self.occupancy.height {
            return true;
        }
        let idx = y * self.occupancy.width + x;
        self.occupancy.npc_tiles[idx] || self.occupancy.enemy_tiles[idx]
    }

    pub fn is_occupied_on_map(&self, map: &Map, x: usize, y: usize) -> bool {
        if x >= map.width || y >= map.height {
            return true;
        }
        self.occupancy.map_id == map.id && self.is_occupied(x, y)
    }

    pub fn apply_event(&mut self, data: &GameData, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Tick => self.tick_counter = self.tick_counter.wrapping_add(1),
            GameEvent::World(world_event) => {
                self.apply_world_event(data, world_event)?;
            }
            GameEvent::Entity(entity_event) => {
                self.apply_entity_event(data, entity_event)?;
            }
            GameEvent::Combat(combat_event) => {
                self.apply_combat_event(combat_event, event)?;
            }
            GameEvent::Movement(_) | GameEvent::Explore(_) | GameEvent::Transition(_) => {
                self.movement.apply_event(event)?;
                if let GameEvent::Movement(MovementEvent::Tick(movement_event, _)) = event {
                    let leader_id = self.leader_id()?;
                    let leader = self.entities.get_mut(leader_id)?;
                    if let Some((dx, dy)) = movement_event.facing {
                        leader.facing = match (dx, dy) {
                            (0, -1) => Direction::Up,
                            (0, 1) => Direction::Down,
                            (-1, 0) => Direction::Left,
                            (1, 0) => Direction::Right,
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

    pub fn reset(&mut self) {
        self.entities.clear();
        self.party.leader_id = 0;
        self.party.companion_ids.clear();
        self.tick_counter = 0;
        self.movement.reset();
        self.combat.reset();
        self.quests.clear();
        self.opened_treasures.clear();
        self.occupancy = OccupancyState::default();
    }

    fn apply_world_event(&mut self, data: &GameData, event: &WorldEvent) -> Result<()> {
        match event {
            WorldEvent::CreateWorld => self.reset(),
            WorldEvent::SetWorldMap(map_id) => {
                self.rebuild_npc_occupancy_for_map(data, map_id)?;
                self.rebuild_enemy_occupancy()?;
            }
            WorldEvent::CreateQuestProgress { quest_id } => {
                self.ensure_quest_progress(quest_id);
            }
            WorldEvent::ChangeQuestCurrentCount { quest_id, delta } => {
                let progress = self.ensure_quest_progress(quest_id);
                progress.current_count = (progress.current_count + *delta).max(0);
                let quest = data.find_quest(quest_id)?;
                progress.current_count = progress.current_count.min(quest.target_count.max(0));
                if progress.current_count >= quest.target_count {
                    progress.completed = true;
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
        Ok(())
    }

    fn apply_entity_event(&mut self, data: &GameData, event: &EntityEvent) -> Result<()> {
        match event {
            EntityEvent::SetLeaderEntity(entity_id) => {
                self.party.leader_id = *entity_id;
            }
            EntityEvent::ClearCompanionEntities => {
                self.party.companion_ids.clear();
            }
            EntityEvent::AddCompanionEntity(entity_id) => {
                if !self.party.companion_ids.contains(entity_id) {
                    self.party.companion_ids.push(*entity_id);
                }
                self.sync_allies_with_party()?;
            }
            EntityEvent::CreateEntity {
                entity_id,
                kind,
                name,
            } => {
                let stat = EntityStat::default();
                ensure!(
                    !self.entities.contains(*entity_id),
                    "Entity already exists: {}",
                    entity_id
                );
                self.entities.upsert(EntityState {
                    id: *entity_id,
                    kind: *kind,
                    name: name.clone(),
                    map_id: String::new(),
                    x: 0,
                    y: 0,
                    facing: Direction::Down,
                    current_hp: stat.base_max_hp,
                    current_mp: stat.base_max_mp,
                    stat,
                    inventory: Vec::new(),
                    loadout: Default::default(),
                });
                self.sync_combat_entry_for_entity(data, *entity_id)?;
                self.sync_allies_with_party()?;
            }
            EntityEvent::SetEntityTransform {
                entity_id,
                map_id,
                position,
                facing,
            } => {
                let previous_tile_index = self.enemy_tile_index_for_entity(*entity_id)?;
                let entity = self.entities.get_mut(*entity_id)?;
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
                if map_id.is_some() || position.is_some() {
                    let next_tile_index = self.enemy_tile_index_for_entity(*entity_id)?;
                    self.apply_enemy_occupancy_delta(previous_tile_index, next_tile_index);
                }
            }
            EntityEvent::SetEntityLevel { entity_id, level } => {
                self.entities.get_mut(*entity_id)?.stat.level = *level;
            }
            EntityEvent::SetEntityExp { entity_id, exp } => {
                self.entities.get_mut(*entity_id)?.stat.exp = *exp;
            }
            EntityEvent::SetEntityExpToNext {
                entity_id,
                exp_to_next,
            } => {
                self.entities.get_mut(*entity_id)?.stat.exp_to_next = *exp_to_next;
            }
            EntityEvent::SetEntityBaseMaxHp {
                entity_id,
                base_max_hp,
            } => {
                let entity = self.entities.get_mut(*entity_id)?;
                entity.stat.base_max_hp = *base_max_hp;
                clamp_entity_resources(entity);
            }
            EntityEvent::SetEntityBaseMaxMp {
                entity_id,
                base_max_mp,
            } => {
                let entity = self.entities.get_mut(*entity_id)?;
                entity.stat.base_max_mp = *base_max_mp;
                clamp_entity_resources(entity);
            }
            EntityEvent::SetEntityBaseAtk {
                entity_id,
                base_atk,
            } => {
                self.entities.get_mut(*entity_id)?.stat.base_atk = *base_atk;
            }
            EntityEvent::SetEntityBaseDef {
                entity_id,
                base_def,
            } => {
                self.entities.get_mut(*entity_id)?.stat.base_def = *base_def;
            }
            EntityEvent::AddEntityExp { entity_id, amount } => {
                let entity = self.entities.get_mut(*entity_id)?;
                entity.stat.add_exp(*amount);
                clamp_entity_resources(entity);
            }
            EntityEvent::ClearEntityInventory { entity_id } => {
                let entity = self.entities.get_mut(*entity_id)?;
                entity.inventory.clear();
                entity.loadout.weapon = None;
                entity.loadout.armor = None;
                entity.loadout.accessory = None;
            }
            EntityEvent::SetEntityLoadoutSlot {
                entity_id,
                slot,
                index,
            } => {
                let entity = self.entities.get_mut(*entity_id)?;
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
            EntityEvent::ChangeEntityItem {
                entity_id,
                item_id,
                delta,
            } => {
                if *delta > 0 {
                    self.add_item_amount(*entity_id, item_id, *delta)?;
                } else if *delta < 0 {
                    self.remove_item_amount(*entity_id, item_id, -*delta)?;
                }
            }
            EntityEvent::SetEntityCurrentHp { entity_id, value } => {
                let entity = self.entities.get_mut(*entity_id)?;
                let max_hp = entity.stat.base_max_hp.max(0);
                entity.current_hp = (*value).clamp(0, max_hp);
            }
            EntityEvent::ChangeEntityHp { entity_id, delta } => {
                let entity = self.entities.get_mut(*entity_id)?;
                entity.current_hp =
                    (entity.current_hp + *delta).clamp(0, entity.stat.base_max_hp.max(0));
            }
            EntityEvent::SetEntityCurrentMp { entity_id, value } => {
                let entity = self.entities.get_mut(*entity_id)?;
                let max_mp = entity.stat.base_max_mp.max(0);
                entity.current_mp = (*value).clamp(0, max_mp);
            }
            EntityEvent::ChangeEntityMp { entity_id, delta } => {
                let entity = self.entities.get_mut(*entity_id)?;
                entity.current_mp =
                    (entity.current_mp + *delta).clamp(0, entity.stat.base_max_mp.max(0));
            }
        }
        Ok(())
    }

    fn apply_combat_event(&mut self, event: &CombatEvent, game_event: &GameEvent) -> Result<()> {
        if self.should_ignore_stale_enemy_combat_event(event) {
            return Ok(());
        }

        let previous_tile_index = match event {
            CombatEvent::MoveEnemy { entity_id, .. } | CombatEvent::RemoveEnemy(entity_id) => {
                self.enemy_tile_index_for_entity(*entity_id)?
            }
            _ => None,
        };

        self.combat.apply_event(game_event)?;
        match event {
            CombatEvent::MoveEnemy { entity_id, x, y } => {
                let enemy_entity = self.entities.get_mut(*entity_id)?;
                enemy_entity.x = *x;
                enemy_entity.y = *y;
                let next_tile_index = self.enemy_tile_index_for_entity(*entity_id)?;
                self.apply_enemy_occupancy_delta(previous_tile_index, next_tile_index);
            }
            CombatEvent::RemoveEnemy(entity_id) => {
                self.entities.remove(*entity_id);
                self.apply_enemy_occupancy_delta(previous_tile_index, None);
            }
            CombatEvent::ClearEnemies => {
                self.clear_enemy_occupancy();
            }
            CombatEvent::SetActive(_)
            | CombatEvent::SetCombatantTimed { .. }
            | CombatEvent::SetRespawnTimer(_)
            | CombatEvent::GrantKillReward { .. } => {}
        }
        Ok(())
    }

    fn should_ignore_stale_enemy_combat_event(&self, event: &CombatEvent) -> bool {
        let Some(entity_id) = combat_event_target_entity_id(event) else {
            return false;
        };

        if self.combat.has_combatant(entity_id) {
            return false;
        }
        if self.party.leader_id == entity_id || self.party.companion_ids.contains(&entity_id) {
            return false;
        }

        true
    }

    fn clear_enemy_occupancy(&mut self) {
        self.occupancy.enemy_tiles.fill(false);
        self.occupancy.enemy_tile_counts.fill(0);
    }

    fn enemy_tile_index_for_entity(&self, entity_id: EntityId) -> Result<Option<usize>> {
        if self.occupancy.width == 0 || self.occupancy.height == 0 {
            return Ok(None);
        }
        let entity = self.entities.get(entity_id)?;
        if entity.map_id != self.occupancy.map_id {
            return Ok(None);
        }
        if entity.x >= self.occupancy.width || entity.y >= self.occupancy.height {
            return Ok(None);
        }
        Ok(Some(entity.y * self.occupancy.width + entity.x))
    }

    fn apply_enemy_occupancy_delta(
        &mut self,
        previous_tile_index: Option<usize>,
        next_tile_index: Option<usize>,
    ) {
        if previous_tile_index == next_tile_index {
            return;
        }

        if let Some(previous_tile_index) = previous_tile_index
            && previous_tile_index < self.occupancy.enemy_tiles.len()
            && let Some(count) = self
                .occupancy
                .enemy_tile_counts
                .get_mut(previous_tile_index)
        {
            if *count > 0 {
                *count -= 1;
            }
            self.occupancy.enemy_tiles[previous_tile_index] = *count > 0;
        }

        if let Some(next_tile_index) = next_tile_index
            && next_tile_index < self.occupancy.enemy_tiles.len()
        {
            if let Some(count) = self.occupancy.enemy_tile_counts.get_mut(next_tile_index) {
                *count = count.saturating_add(1);
            }
            self.occupancy.enemy_tiles[next_tile_index] = true;
        }
    }

    fn rebuild_npc_occupancy_for_map(&mut self, data: &GameData, map_id: &str) -> Result<()> {
        self.occupancy.map_id = map_id.into();
        let map = data.find_map(map_id)?;
        self.occupancy.width = map.width;
        self.occupancy.height = map.height;
        let len = map.width * map.height;
        self.occupancy.npc_tiles = vec![false; len];
        self.occupancy.enemy_tiles = vec![false; len];
        self.occupancy.enemy_tile_counts = vec![0; len];

        for (x, y, _) in &map.npcs {
            if *x < map.width && *y < map.height {
                self.occupancy.npc_tiles[*y * map.width + *x] = true;
            }
        }
        Ok(())
    }

    fn rebuild_enemy_occupancy(&mut self) -> Result<()> {
        self.clear_enemy_occupancy();
        if self.occupancy.width == 0 || self.occupancy.height == 0 {
            return Ok(());
        }
        for enemy in &self.combat.enemies {
            let entity = self.entities.get(enemy.entity_id)?;
            if entity.current_hp <= 0 {
                continue;
            }
            if entity.map_id != self.occupancy.map_id {
                continue;
            }
            if entity.x < self.occupancy.width && entity.y < self.occupancy.height {
                let index = entity.y * self.occupancy.width + entity.x;
                if let Some(count) = self.occupancy.enemy_tile_counts.get_mut(index) {
                    *count = count.saturating_add(1);
                    self.occupancy.enemy_tiles[index] = true;
                }
            }
        }
        Ok(())
    }

    fn sync_allies_with_party(&mut self) -> Result<()> {
        let mut party_ids = Vec::with_capacity(1 + self.party.companion_ids.len());
        let mut party_set = BTreeSet::new();
        if self.party.leader_id != 0 && self.entities.contains(self.party.leader_id) {
            party_ids.push(self.party.leader_id);
            party_set.insert(self.party.leader_id);
        }
        for entity_id in &self.party.companion_ids {
            self.entities.get(*entity_id)?;
            if party_set.insert(*entity_id) {
                party_ids.push(*entity_id);
            }
        }

        self.combat
            .allies
            .retain(|ally| party_set.contains(&ally.entity_id));

        let mut existing_allies = BTreeSet::new();
        for ally in &self.combat.allies {
            existing_allies.insert(ally.entity_id);
        }

        for entity_id in party_ids {
            if !existing_allies.contains(&entity_id) {
                self.combat.allies.push(AllyCombatantState {
                    entity_id,
                    combatant: CombatantState::default(),
                });
                existing_allies.insert(entity_id);
            }
        }
        Ok(())
    }

    fn sync_combat_entry_for_entity(&mut self, data: &GameData, entity_id: EntityId) -> Result<()> {
        let entity = self.entities.get(entity_id)?;

        if matches!(entity.kind, EntityKind::Enemy) {
            let source_enemy_id = data
                .find_enemy(&entity.name)
                .or_else(|_| data.find_enemy_by_name(&entity.name))
                .map(|enemy| enemy.id.clone())?;

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
        Ok(())
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

fn clamp_entity_resources(entity: &mut EntityState) {
    entity.current_hp = entity.current_hp.clamp(0, entity.stat.base_max_hp.max(0));
    entity.current_mp = entity.current_mp.clamp(0, entity.stat.base_max_mp.max(0));
}

fn combat_event_target_entity_id(event: &CombatEvent) -> Option<EntityId> {
    match event {
        CombatEvent::MoveEnemy { entity_id, .. }
        | CombatEvent::SetCombatantTimed { entity_id, .. } => Some(*entity_id),
        CombatEvent::RemoveEnemy(entity_id) => Some(*entity_id),
        CombatEvent::SetActive(_)
        | CombatEvent::ClearEnemies
        | CombatEvent::SetRespawnTimer(_)
        | CombatEvent::GrantKillReward { .. } => None,
    }
}

impl GameEventSubscriber for WorldState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(
            kind,
            GameEventKind::World
                | GameEventKind::Entity
                | GameEventKind::Combat
                | GameEventKind::Movement
                | GameEventKind::Explore
                | GameEventKind::Transition
        )
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec, vec::Vec};

    use anyhow::{Result, anyhow};

    use super::{OccupancyState, WorldState};
    use crate::{
        data::{Direction, Quest, QuestType},
        game::{
            game_data::GameData,
            game_event::{CombatEvent, EntityEvent, GameEvent, WorldEvent},
            state::{
                CombatantState, EnemyCombatantState, EntityKind, EntityState, ItemStack, TimedKind,
            },
        },
    };

    #[test]
    fn add_entity_exp_keeps_runtime_resources_on_entity() -> Result<()> {
        let data = GameData::default();
        let mut world = WorldState::empty();

        world.apply_event(&data, &GameEvent::World(WorldEvent::CreateWorld))?;
        world.apply_event(&data, &GameEvent::Entity(EntityEvent::SetLeaderEntity(1)))?;
        world.apply_event(
            &data,
            &GameEvent::Entity(EntityEvent::CreateEntity {
                entity_id: 1,
                kind: EntityKind::Player,
                name: String::from("Hero"),
            }),
        )?;
        world.apply_event(
            &data,
            &GameEvent::Entity(EntityEvent::AddEntityExp {
                entity_id: 1,
                amount: 100,
            }),
        )?;

        let entity = world.entity(1)?;
        let combatant = world.combat.combatant(1)?;

        assert_eq!(entity.stat.level, 2);
        assert_eq!(entity.current_hp, 80);
        assert_eq!(entity.current_mp, 30);
        assert!(combatant.timed.effects.is_empty());
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
    fn enemy_occupancy_updates_on_remove_enemy() -> Result<()> {
        let data = GameData::default();
        let mut world = WorldState::empty();

        world.occupancy = OccupancyState {
            map_id: String::from("map"),
            width: 3,
            height: 3,
            npc_tiles: vec![false; 9],
            enemy_tiles: vec![false; 9],
            enemy_tile_counts: vec![0; 9],
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
            current_hp: 80,
            current_mp: 0,
            inventory: Vec::<ItemStack>::new(),
            loadout: Default::default(),
        });
        world.combat.enemies.push(EnemyCombatantState {
            entity_id: 10,
            source_enemy_id: String::from("slime"),
            combatant: CombatantState::default(),
        });
        world.rebuild_enemy_occupancy()?;
        assert!(world.is_occupied(1, 1));

        world.apply_event(
            &data,
            &GameEvent::Entity(EntityEvent::ChangeEntityHp {
                entity_id: 10,
                delta: -75,
            }),
        )?;
        assert!(world.is_occupied(1, 1));

        world.apply_event(
            &data,
            &GameEvent::Entity(EntityEvent::ChangeEntityHp {
                entity_id: 10,
                delta: -5,
            }),
        )?;
        assert!(world.is_occupied(1, 1));

        world.apply_event(&data, &GameEvent::Combat(CombatEvent::RemoveEnemy(10)))?;
        assert!(!world.is_occupied(1, 1));
        Ok(())
    }

    #[test]
    fn stale_enemy_combat_event_after_remove_is_ignored() -> Result<()> {
        let data = GameData::default();
        let mut world = WorldState::empty();

        world.occupancy = OccupancyState {
            map_id: String::from("map"),
            width: 3,
            height: 3,
            npc_tiles: vec![false; 9],
            enemy_tiles: vec![false; 9],
            enemy_tile_counts: vec![0; 9],
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
            current_hp: 80,
            current_mp: 0,
            inventory: Vec::<ItemStack>::new(),
            loadout: Default::default(),
        });
        world.combat.enemies.push(EnemyCombatantState {
            entity_id: 10,
            source_enemy_id: String::from("slime"),
            combatant: CombatantState::default(),
        });

        world.apply_event(&data, &GameEvent::Combat(CombatEvent::RemoveEnemy(10)))?;
        world.apply_event(
            &data,
            &GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id: 10,
                kind: TimedKind::AttackCooldown,
                end_tick: 5,
            }),
        )?;

        Ok(())
    }
}

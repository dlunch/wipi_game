use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::QuestProgress;
use crate::game::state::{
    CombatState, EntityId, EntityState, EntityStore, GOLD_ITEM_ID, PartyState,
};
use crate::game::{
    CombatEvent, CombatSpawnKind, GameData, GameEvent, GameEventKind, GameEventSubscriber,
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
            WorldEvent::SetParty(party) => {
                self.party = party.clone();
            }
            WorldEvent::UpsertEntity(entity) => {
                self.entities.upsert(entity.clone());
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
            WorldEvent::SetEntityMap { entity_id, map_id } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.map_id = map_id.clone();
                }
            }
            WorldEvent::SetEntityPosition { entity_id, x, y } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.x = *x;
                    entity.y = *y;
                }
                self.rebuild_enemy_occupancy();
            }
            WorldEvent::SetEntityFacing { entity_id, facing } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.facing = *facing;
                }
            }
            WorldEvent::SetEntityStat { entity_id, stat } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.stat = *stat;
                }
            }
            WorldEvent::SetEntityInventory {
                entity_id,
                inventory,
            } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.inventory = inventory.clone();
                }
            }
            WorldEvent::SetEntityLoadout { entity_id, loadout } => {
                if let Some(entity) = self.entities.get_mut(*entity_id) {
                    entity.loadout = *loadout;
                }
            }
            WorldEvent::AddEntityItem {
                entity_id,
                item_id,
                amount,
            } => self.add_item_amount(*entity_id, item_id, *amount),
            WorldEvent::RemoveEntityItem {
                entity_id,
                item_id,
                amount,
            } => self.remove_item_amount(*entity_id, item_id, *amount),
            WorldEvent::AddQuestProgress(progress) => {
                if let Some(existing) = self
                    .quests
                    .iter_mut()
                    .find(|quest| quest.quest_id == progress.quest_id)
                {
                    *existing = progress.clone();
                } else {
                    self.quests.push(progress.clone());
                }
            }
            WorldEvent::AddOpenedTreasure { map_id, x, y } => {
                if !self.is_treasure_opened(map_id, *x, *y) {
                    self.opened_treasures.push((map_id.clone(), *x, *y));
                }
            }
            WorldEvent::ResetMovement => {
                self.movement = MovementState::default();
            }
            WorldEvent::ResetCombat => {
                self.combat = CombatState::default();
                self.clear_enemy_occupancy();
            }
        }
    }

    fn apply_combat_event(&mut self, event: &CombatEvent, game_event: &GameEvent) -> Result<()> {
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
            CombatEvent::SpawnEntity {
                kind: CombatSpawnKind::Enemy { .. },
                ..
            }
            | CombatEvent::ClearEnemies
            | CombatEvent::SetCombatantStats { .. }
            | CombatEvent::SetCombatantTimed { .. } => {
                self.rebuild_enemy_occupancy();
            }
            CombatEvent::SetActive(_)
            | CombatEvent::ClearAllies
            | CombatEvent::SpawnEntity {
                kind: CombatSpawnKind::Ally,
                ..
            }
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

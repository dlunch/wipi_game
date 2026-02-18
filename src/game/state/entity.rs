use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::Direction;

pub type EntityId = u32;
pub const GOLD_ITEM_ID: &str = "gold";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Player,
    Companion,
    Enemy,
    Npc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityStat {
    pub level: i32,
    pub exp: i32,
    pub exp_to_next: i32,
    pub base_max_hp: i32,
    pub base_max_mp: i32,
    pub base_atk: i32,
    pub base_def: i32,
}

impl Default for EntityStat {
    fn default() -> Self {
        Self {
            level: 1,
            exp: 0,
            exp_to_next: 100,
            base_max_hp: 80,
            base_max_mp: 30,
            base_atk: 12,
            base_def: 8,
        }
    }
}

impl EntityStat {
    pub fn add_exp(&mut self, exp: i32) -> bool {
        self.exp += exp.max(0);
        let mut leveled_up = false;
        while self.exp >= self.exp_to_next {
            self.exp -= self.exp_to_next;
            self.level += 1;
            self.exp_to_next = self.level * 100;
            self.base_max_hp += 10;
            self.base_max_mp += 5;
            self.base_atk += 2;
            self.base_def += 1;
            leveled_up = true;
        }
        leveled_up
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item_id: String,
    pub amount: i32,
}

impl ItemStack {
    pub fn new(item_id: impl Into<String>, amount: i32) -> Self {
        Self {
            item_id: item_id.into(),
            amount,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoadoutState {
    pub weapon: Option<usize>,
    pub armor: Option<usize>,
    pub accessory: Option<usize>,
}

#[derive(Debug)]
pub struct EntityState {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    pub map_id: String,
    pub x: usize,
    pub y: usize,
    pub facing: Direction,
    pub stat: EntityStat,
    pub inventory: Vec<ItemStack>,
    pub loadout: LoadoutState,
}

impl EntityState {
    pub fn new_player(id: EntityId, name: String, map_id: String) -> Self {
        Self {
            id,
            kind: EntityKind::Player,
            name,
            map_id,
            x: 0,
            y: 0,
            facing: Direction::Down,
            stat: EntityStat::default(),
            inventory: Vec::new(),
            loadout: LoadoutState::default(),
        }
    }
}

#[derive(Debug)]
pub struct EntityStore {
    list: Vec<EntityState>,
    index_by_id: BTreeMap<EntityId, usize>,
    pub next_entity_id: EntityId,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            list: Vec::new(),
            index_by_id: BTreeMap::new(),
            next_entity_id: 1,
        }
    }

    pub fn from_list(list: Vec<EntityState>, next_entity_id: EntityId) -> Self {
        let mut index_by_id = BTreeMap::new();
        for (index, entity) in list.iter().enumerate() {
            index_by_id.insert(entity.id, index);
        }
        Self {
            list,
            index_by_id,
            next_entity_id: next_entity_id.max(1),
        }
    }

    pub fn iter(&self) -> core::slice::Iter<'_, EntityState> {
        self.list.iter()
    }

    pub fn upsert(&mut self, entity: EntityState) {
        let next = entity.id.wrapping_add(1).max(1);
        if next > self.next_entity_id {
            self.next_entity_id = next;
        }

        if let Some(index) = self.index_by_id.get(&entity.id).copied() {
            self.list[index] = entity;
        } else {
            self.list.push(entity);
            let index = self.list.len() - 1;
            let entity_id = self.list[index].id;
            self.index_by_id.insert(entity_id, index);
        }
    }

    pub fn get(&self, entity_id: EntityId) -> Result<&EntityState> {
        let index = self
            .index_by_id
            .get(&entity_id)
            .copied()
            .ok_or_else(|| anyhow!("Entity id not found: {}", entity_id))?;
        self.list
            .get(index)
            .ok_or_else(|| anyhow!("Entity index out of bounds: {}", index))
    }

    pub fn get_mut(&mut self, entity_id: EntityId) -> Result<&mut EntityState> {
        let index = self
            .index_by_id
            .get(&entity_id)
            .copied()
            .ok_or_else(|| anyhow!("Entity id not found: {}", entity_id))?;
        self.list
            .get_mut(index)
            .ok_or_else(|| anyhow!("Entity index out of bounds: {}", index))
    }

    pub fn contains(&self, entity_id: EntityId) -> bool {
        self.index_by_id.contains_key(&entity_id)
    }

    pub fn remove(&mut self, entity_id: EntityId) {
        let Some(index) = self.index_by_id.remove(&entity_id) else {
            return;
        };

        let last_index = self.list.len() - 1;
        self.list.swap_remove(index);
        if index != last_index
            && let Some(moved) = self.list.get(index)
        {
            self.index_by_id.insert(moved.id, index);
        }
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct PartyState {
    pub leader_id: EntityId,
    pub companion_ids: Vec<EntityId>,
}

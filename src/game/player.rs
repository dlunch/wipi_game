use alloc::string::String;
use alloc::vec::Vec;

use super::combat::Direction;
use crate::data::{Item, ItemKind, Map, PlayerStats, QuestProgress};

pub struct Player {
    pub name: String,
    pub stats: PlayerStats,
    pub inventory: Vec<Item>,
    pub equipped_weapon: Option<usize>,
    pub equipped_armor: Option<usize>,
    pub equipped_accessory: Option<usize>,
    pub current_map_id: String,
    pub x: usize,
    pub y: usize,
    pub facing: Direction,
    pub quests: Vec<QuestProgress>,
    pub opened_treasures: Vec<(String, usize, usize)>,
    pub skill_cooldowns: [u32; 3],
}

impl Player {
    pub fn new(name: String, start_map: &str) -> Self {
        Self {
            name,
            stats: PlayerStats::default(),
            inventory: Vec::new(),
            equipped_weapon: None,
            equipped_armor: None,
            equipped_accessory: None,
            current_map_id: start_map.into(),
            x: 0,
            y: 0,
            facing: Direction::Down,
            quests: Vec::new(),
            opened_treasures: Vec::new(),
            skill_cooldowns: [0; 3],
        }
    }

    pub fn update_cooldowns(&mut self) {
        for cd in &mut self.skill_cooldowns {
            if *cd > 0 {
                *cd -= 1;
            }
        }
    }

    pub fn can_use_skill(&self, slot: usize, mp_cost: i32) -> bool {
        slot < 3 && self.skill_cooldowns[slot] == 0 && self.stats.current_mp >= mp_cost
    }

    pub fn use_skill(&mut self, slot: usize, mp_cost: i32, cooldown: u32) {
        if slot < 3 {
            self.skill_cooldowns[slot] = cooldown;
            self.stats.current_mp = (self.stats.current_mp - mp_cost).max(0);
        }
    }

    pub fn is_treasure_opened(&self, map_id: &str, x: usize, y: usize) -> bool {
        self.opened_treasures
            .iter()
            .any(|(m, tx, ty)| m == map_id && *tx == x && *ty == y)
    }

    pub fn open_treasure(&mut self, map_id: &str, x: usize, y: usize) {
        if !self.is_treasure_opened(map_id, x, y) {
            self.opened_treasures.push((map_id.into(), x, y));
        }
    }

    pub fn spawn_at_map(&mut self, map: &Map) {
        if let Some((x, y)) = map.find_player_start() {
            self.x = x;
            self.y = y;
        }
        self.current_map_id = map.id.clone();
    }

    pub fn get_weapon(&self) -> Option<&Item> {
        self.equipped_weapon.and_then(|i| self.inventory.get(i))
    }

    pub fn get_armor(&self) -> Option<&Item> {
        self.equipped_armor.and_then(|i| self.inventory.get(i))
    }

    pub fn get_accessory(&self) -> Option<&Item> {
        self.equipped_accessory.and_then(|i| self.inventory.get(i))
    }

    pub fn total_atk(&self) -> i32 {
        self.stats
            .total_atk(self.get_weapon(), self.get_accessory())
    }

    pub fn total_def(&self) -> i32 {
        self.stats.total_def(self.get_armor(), self.get_accessory())
    }

    pub fn add_item(&mut self, item: Item) {
        self.inventory.push(item);
    }

    pub fn use_item(&mut self, index: usize) -> bool {
        if index >= self.inventory.len() {
            return false;
        }

        let item = &self.inventory[index];
        match item.kind {
            ItemKind::Consumable => {
                let heal = item.param1;
                self.stats.heal(heal);
                self.inventory.remove(index);
                self.fix_equipped_indices(index);
                true
            }
            ItemKind::Weapon => {
                self.equipped_weapon = Some(index);
                true
            }
            ItemKind::Armor => {
                self.equipped_armor = Some(index);
                true
            }
            ItemKind::Accessory => {
                self.equipped_accessory = Some(index);
                true
            }
        }
    }

    fn fix_equipped_indices(&mut self, removed: usize) {
        if let Some(ref mut i) = self.equipped_weapon {
            if *i > removed {
                *i -= 1;
            } else if *i == removed {
                self.equipped_weapon = None;
            }
        }
        if let Some(ref mut i) = self.equipped_armor {
            if *i > removed {
                *i -= 1;
            } else if *i == removed {
                self.equipped_armor = None;
            }
        }
        if let Some(ref mut i) = self.equipped_accessory {
            if *i > removed {
                *i -= 1;
            } else if *i == removed {
                self.equipped_accessory = None;
            }
        }
    }

    pub fn can_move(&self, map: &Map, dx: i32, dy: i32) -> bool {
        let Some(new_x) = self.x.checked_add_signed(dx as isize) else {
            return false;
        };
        let Some(new_y) = self.y.checked_add_signed(dy as isize) else {
            return false;
        };
        map.get_tile(new_x, new_y).is_passable()
    }

    pub fn move_by(&mut self, dx: i32, dy: i32) {
        if let Some(new_x) = self.x.checked_add_signed(dx as isize) {
            self.x = new_x;
        }
        if let Some(new_y) = self.y.checked_add_signed(dy as isize) {
            self.y = new_y;
        }
        self.set_facing(dx, dy);
    }

    pub fn set_facing(&mut self, dx: i32, dy: i32) {
        self.facing = match (dx, dy) {
            (0, -1) => Direction::Up,
            (0, 1) => Direction::Down,
            (-1, 0) => Direction::Left,
            (1, 0) => Direction::Right,
            _ => self.facing,
        };
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

    pub fn add_quest(&mut self, quest_id: &str) {
        if !self.has_quest(quest_id) {
            self.quests.push(QuestProgress {
                quest_id: quest_id.into(),
                current_count: 0,
                completed: false,
                rewarded: false,
            });
        }
    }

    pub fn complete_quest(&mut self, quest_id: &str) {
        if let Some(q) = self.quests.iter_mut().find(|q| q.quest_id == quest_id) {
            q.rewarded = true;
        }
    }

    pub fn has_item(&self, item_id: &str) -> bool {
        self.inventory.iter().any(|i| i.id == item_id)
    }

    pub fn remove_item(&mut self, item_id: &str) -> bool {
        if let Some(idx) = self.inventory.iter().position(|i| i.id == item_id) {
            self.inventory.remove(idx);
            self.fix_equipped_indices(idx);
            true
        } else {
            false
        }
    }

    pub fn remove_item_at(&mut self, index: usize) -> Option<Item> {
        if index >= self.inventory.len() {
            return None;
        }
        let item = self.inventory.remove(index);
        self.fix_equipped_indices(index);
        Some(item)
    }
}

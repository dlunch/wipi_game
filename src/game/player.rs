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
    pub opened_treasures: Vec<(String, usize, usize)>, // (map_id, x, y)
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

    pub fn total_atk(&self) -> i32 {
        self.stats.total_atk(self.get_weapon())
    }

    pub fn total_def(&self) -> i32 {
        self.stats.total_def(self.get_armor())
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
        let new_x = (self.x as i32 + dx) as usize;
        let new_y = (self.y as i32 + dy) as usize;
        map.get_tile(new_x, new_y).is_passable()
    }

    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.x = (self.x as i32 + dx) as usize;
        self.y = (self.y as i32 + dy) as usize;

        self.facing = match (dx, dy) {
            (0, -1) => Direction::Up,
            (0, 1) => Direction::Down,
            (-1, 0) => Direction::Left,
            (1, 0) => Direction::Right,
            _ => self.facing,
        };
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
}

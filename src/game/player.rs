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
                let heal = item.hp_restore();
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

    pub fn mark_quest_rewarded(&mut self, quest_id: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Item, ItemKind};

    fn make_item(id: &str, kind: ItemKind) -> Item {
        Item {
            id: String::from(id),
            name: String::from(id),
            kind,
            param1: 10,
            param2: 5,
            param3: 0,
            price: 100,
        }
    }

    fn make_potion() -> Item {
        Item {
            id: String::from("potion"),
            name: String::from("Potion"),
            kind: ItemKind::Consumable,
            param1: 30,
            param2: 0,
            param3: 0,
            price: 50,
        }
    }

    #[test]
    fn new_player_starts_empty() {
        let player = Player::new(String::from("Test"), "village");
        assert_eq!(player.name, "Test");
        assert_eq!(player.current_map_id, "village");
        assert!(player.inventory.is_empty());
        assert!(player.equipped_weapon.is_none());
        assert!(player.quests.is_empty());
    }

    #[test]
    fn equip_weapon_via_use_item() {
        let mut player = Player::new(String::from("H"), "v");
        player.add_item(make_item("sword", ItemKind::Weapon));
        assert!(player.use_item(0));
        assert_eq!(player.equipped_weapon, Some(0));
    }

    #[test]
    fn equip_armor_via_use_item() {
        let mut player = Player::new(String::from("H"), "v");
        player.add_item(make_item("armor", ItemKind::Armor));
        assert!(player.use_item(0));
        assert_eq!(player.equipped_armor, Some(0));
    }

    #[test]
    fn use_consumable_heals_and_removes() {
        let mut player = Player::new(String::from("H"), "v");
        player.stats.current_hp = 20;
        player.add_item(make_potion());
        assert!(player.use_item(0));
        assert_eq!(player.stats.current_hp, 50);
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn fix_equipped_indices_on_remove() {
        let mut player = Player::new(String::from("H"), "v");
        player.add_item(make_potion());
        player.add_item(make_item("sword", ItemKind::Weapon));
        player.add_item(make_item("armor", ItemKind::Armor));
        player.equipped_weapon = Some(1);
        player.equipped_armor = Some(2);

        player.use_item(0); // remove potion at index 0
        assert_eq!(player.equipped_weapon, Some(0)); // shifted from 1 to 0
        assert_eq!(player.equipped_armor, Some(1)); // shifted from 2 to 1
    }

    #[test]
    fn fix_equipped_clears_on_exact_removal() {
        let mut player = Player::new(String::from("H"), "v");
        player.add_item(make_item("sword", ItemKind::Weapon));
        player.equipped_weapon = Some(0);

        player.remove_item("sword");
        assert_eq!(player.equipped_weapon, None);
    }

    #[test]
    fn use_item_out_of_bounds() {
        let mut player = Player::new(String::from("H"), "v");
        assert!(!player.use_item(0));
        assert!(!player.use_item(99));
    }

    #[test]
    fn remove_item_at_returns_item() {
        let mut player = Player::new(String::from("H"), "v");
        player.add_item(make_potion());
        let removed = player.remove_item_at(0);
        assert_eq!(removed.as_ref().map(|i| i.id.as_str()), Some("potion"));
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn remove_item_at_out_of_bounds() {
        let mut player = Player::new(String::from("H"), "v");
        assert!(player.remove_item_at(0).is_none());
    }

    #[test]
    fn quest_lifecycle() {
        let mut player = Player::new(String::from("H"), "v");
        assert!(!player.has_quest("q1"));

        player.add_quest("q1");
        assert!(player.has_quest("q1"));
        assert!(!player.is_quest_complete("q1"));

        player.quests[0].completed = true;
        assert!(player.is_quest_complete("q1"));

        player.mark_quest_rewarded("q1");
        assert!(!player.has_quest("q1")); // rewarded quests are not "active"
    }

    #[test]
    fn add_quest_no_duplicates() {
        let mut player = Player::new(String::from("H"), "v");
        player.add_quest("q1");
        player.add_quest("q1");
        assert_eq!(player.quests.len(), 1);
    }

    #[test]
    fn treasure_tracking() {
        let mut player = Player::new(String::from("H"), "v");
        assert!(!player.is_treasure_opened("map1", 3, 4));

        player.open_treasure("map1", 3, 4);
        assert!(player.is_treasure_opened("map1", 3, 4));
        assert!(!player.is_treasure_opened("map1", 3, 5));
        assert!(!player.is_treasure_opened("map2", 3, 4));

        player.open_treasure("map1", 3, 4); // duplicate
        assert_eq!(player.opened_treasures.len(), 1);
    }

    #[test]
    fn skill_cooldowns() {
        let mut player = Player::new(String::from("H"), "v");
        assert!(player.can_use_skill(0, 10));

        player.use_skill(0, 10, 30);
        assert!(!player.can_use_skill(0, 10)); // on cooldown
        assert_eq!(player.skill_cooldowns[0], 30);
        assert_eq!(player.stats.current_mp, 20); // 30 - 10

        for _ in 0..30 {
            player.update_cooldowns();
        }
        assert!(player.can_use_skill(0, 10)); // cooldown expired
    }

    #[test]
    fn skill_insufficient_mp() {
        let mut player = Player::new(String::from("H"), "v");
        player.stats.current_mp = 5;
        assert!(!player.can_use_skill(0, 10));
    }

    #[test]
    fn total_atk_def_with_equipment() {
        let mut player = Player::new(String::from("H"), "v");
        let base_atk = player.stats.base_atk;
        let base_def = player.stats.base_def;
        assert_eq!(player.total_atk(), base_atk);
        assert_eq!(player.total_def(), base_def);

        player.add_item(make_item("sword", ItemKind::Weapon));
        player.equipped_weapon = Some(0);
        assert_eq!(player.total_atk(), base_atk + 10);

        player.add_item(make_item("armor", ItemKind::Armor));
        player.equipped_armor = Some(1);
        assert_eq!(player.total_def(), base_def + 10);
    }

    #[test]
    fn has_item_and_remove_item() {
        let mut player = Player::new(String::from("H"), "v");
        player.add_item(make_potion());
        assert!(player.has_item("potion"));

        assert!(player.remove_item("potion"));
        assert!(!player.has_item("potion"));
        assert!(!player.remove_item("potion")); // already removed
    }
}

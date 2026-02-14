use alloc::string::String;
use alloc::vec::Vec;

use crate::data::{Item, Map, PlayerStats, QuestProgress};
use crate::game::Direction;

#[derive(Debug)]
pub struct PlayerState {
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
    pub mp_regen_timer: u32,
}

impl PlayerState {
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
            mp_regen_timer: 0,
        }
    }

    pub fn is_treasure_opened(&self, map_id: &str, x: usize, y: usize) -> bool {
        self.opened_treasures
            .iter()
            .any(|(m, tx, ty)| m == map_id && *tx == x && *ty == y)
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

    pub fn can_move(&self, map: &Map, dx: i32, dy: i32) -> bool {
        let Some(new_x) = self.x.checked_add_signed(dx as isize) else {
            return false;
        };
        let Some(new_y) = self.y.checked_add_signed(dy as isize) else {
            return false;
        };
        map.get_tile(new_x, new_y).is_passable()
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

    pub fn has_item(&self, item_id: &str) -> bool {
        self.inventory.iter().any(|i| i.id == item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Item, ItemKind, QuestProgress};

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
        let player = PlayerState::new(String::from("Test"), "village");
        assert_eq!(player.name, "Test");
        assert_eq!(player.current_map_id, "village");
        assert!(player.inventory.is_empty());
        assert!(player.equipped_weapon.is_none());
        assert!(player.quests.is_empty());
    }

    #[test]
    fn quest_lifecycle() {
        let mut player = PlayerState::new(String::from("H"), "v");
        assert!(!player.has_quest("q1"));

        player.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 0,
            completed: false,
            rewarded: false,
        });
        assert!(player.has_quest("q1"));
        assert!(!player.is_quest_complete("q1"));

        player.quests[0].completed = true;
        assert!(player.is_quest_complete("q1"));

        player.quests[0].rewarded = true;
        assert!(!player.has_quest("q1")); // rewarded quests are not "active"
    }

    #[test]
    fn has_quest_ignores_rewarded_quest() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 0,
            completed: false,
            rewarded: true,
        });
        assert!(!player.has_quest("q1"));
    }

    #[test]
    fn treasure_tracking() {
        let mut player = PlayerState::new(String::from("H"), "v");
        assert!(!player.is_treasure_opened("map1", 3, 4));

        player.opened_treasures.push((String::from("map1"), 3, 4));
        assert!(player.is_treasure_opened("map1", 3, 4));
        assert!(!player.is_treasure_opened("map1", 3, 5));
        assert!(!player.is_treasure_opened("map2", 3, 4));
    }

    #[test]
    fn total_atk_def_with_equipment() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let base_atk = player.stats.base_atk;
        let base_def = player.stats.base_def;
        assert_eq!(player.total_atk(), base_atk);
        assert_eq!(player.total_def(), base_def);

        player.inventory.push(make_item("sword", ItemKind::Weapon));
        player.equipped_weapon = Some(0);
        assert_eq!(player.total_atk(), base_atk + 10);

        player.inventory.push(make_item("armor", ItemKind::Armor));
        player.equipped_armor = Some(1);
        assert_eq!(player.total_def(), base_def + 10);
    }

    #[test]
    fn has_item_query() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.inventory.push(make_potion());
        assert!(player.has_item("potion"));
        assert!(!player.has_item("ether"));
    }
}

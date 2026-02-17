use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::{Direction, Item, PlayerStats};
use crate::game::{
    GameData, GameEvent, GameEventKind, GameEventSubscriber, MovementEvent, WorldEvent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileEvent {
    Treasure,
    MapExit(String),
    DungeonEntrance(String),
}

#[derive(Debug, Clone)]
pub struct CharacterState {
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
}

impl CharacterState {
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
        }
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

    pub fn has_item(&self, item_id: &str) -> bool {
        self.inventory.iter().any(|i| i.id == item_id)
    }

    pub fn can_use_skill(&self, cooldowns: &[u32; 3], slot: usize, mp_cost: i32) -> bool {
        slot < 3 && cooldowns[slot] == 0 && self.stats.current_mp >= mp_cost
    }
}

impl GameEventSubscriber for CharacterState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(kind, GameEventKind::World | GameEventKind::Movement)
    }
}

impl CharacterState {
    pub fn apply_event(&mut self, _data: &GameData, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::World(session_event) => match session_event {
                WorldEvent::Create => {}
                WorldEvent::SetPlayerName(name) => {
                    self.name = name.clone();
                }
                WorldEvent::SetPlayerStats(stats) => {
                    self.stats = stats.clone();
                }
                WorldEvent::SetPlayerInventory(inventory) => {
                    self.inventory = inventory.clone();
                }
                WorldEvent::SetPlayerMap(map_id) => {
                    self.current_map_id = map_id.clone();
                }
                WorldEvent::SetPlayerPosition { x, y } => {
                    self.x = *x;
                    self.y = *y;
                }
                WorldEvent::SetPlayerFacing(facing) => {
                    self.facing = *facing;
                }
                WorldEvent::AddPlayerItem(item) => {
                    self.inventory.push(item.clone());
                }
                WorldEvent::SetEquippedWeapon(index) => {
                    self.equipped_weapon = *index;
                }
                WorldEvent::SetEquippedArmor(index) => {
                    self.equipped_armor = *index;
                }
                WorldEvent::SetEquippedAccessory(index) => {
                    self.equipped_accessory = *index;
                }
                WorldEvent::AddQuestProgress(_)
                | WorldEvent::AddOpenedTreasure { .. }
                | WorldEvent::SetSkillCooldowns(_)
                | WorldEvent::SetMpRegenTimer(_)
                | WorldEvent::ResetMovement
                | WorldEvent::ResetCombat => {}
            },
            GameEvent::Movement(MovementEvent::Tick(movement_event, _)) => {
                if let Some((dx, dy)) = movement_event.facing {
                    self.facing = match (dx, dy) {
                        (0, -1) => Direction::Up,
                        (0, 1) => Direction::Down,
                        (-1, 0) => Direction::Left,
                        (1, 0) => Direction::Right,
                        _ => self.facing,
                    };
                }
                if let Some((dx, dy)) = movement_event.step {
                    self.x = (self.x as i32 + dx) as usize;
                    self.y = (self.y as i32 + dy) as usize;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::data::{Item, ItemKind};
    use crate::game::{GameEvent, WorldEvent};

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

    #[test]
    fn new_player_starts_empty() {
        let player = CharacterState::new(String::from("Test"), "village");
        assert_eq!(player.name, "Test");
        assert_eq!(player.current_map_id, "village");
        assert!(player.inventory.is_empty());
        assert!(player.equipped_weapon.is_none());
    }

    #[test]
    fn total_atk_def_with_equipment() {
        let mut player = CharacterState::new(String::from("H"), "v");
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
        let mut player = CharacterState::new(String::from("H"), "v");
        player
            .inventory
            .push(make_item("potion", ItemKind::Consumable));
        assert!(player.has_item("potion"));
        assert!(!player.has_item("ether"));
    }

    #[test]
    fn apply_event_sets_equipment_and_inventory() -> Result<()> {
        let mut player = CharacterState::new(String::from("H"), "v");
        let inventory = vec![
            make_item("sword", ItemKind::Weapon),
            make_item("armor", ItemKind::Armor),
        ];
        player.apply_event(
            &GameData::default(),
            &GameEvent::World(WorldEvent::SetPlayerInventory(inventory)),
        )?;
        player.apply_event(
            &GameData::default(),
            &GameEvent::World(WorldEvent::SetEquippedWeapon(Some(0))),
        )?;
        player.apply_event(
            &GameData::default(),
            &GameEvent::World(WorldEvent::SetEquippedArmor(Some(1))),
        )?;
        assert_eq!(player.equipped_weapon, Some(0));
        assert_eq!(player.equipped_armor, Some(1));
        assert_eq!(player.inventory.len(), 2);
        Ok(())
    }

    #[test]
    fn skill_cooldowns() {
        let mut player = CharacterState::new(String::from("H"), "v");
        let cooldowns = [0; 3];
        assert!(player.can_use_skill(&cooldowns, 0, 10));

        let cooldowns = [30, 0, 0];
        player.stats.current_mp = 20;
        assert!(!player.can_use_skill(&cooldowns, 0, 10));

        let cooldowns = [0, 0, 0];
        assert!(player.can_use_skill(&cooldowns, 0, 10));
    }

    #[test]
    fn skill_insufficient_mp() {
        let mut player = CharacterState::new(String::from("H"), "v");
        player.stats.current_mp = 5;
        assert!(!player.can_use_skill(&[0; 3], 0, 10));
    }
}

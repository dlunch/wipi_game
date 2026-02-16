use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::{Direction, Item, ItemKind, PlayerStats};
use crate::game::state::combat::KillReward;
use crate::game::{GameData, GameEvent, SessionEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileEvent {
    Treasure,
    MapExit(String),
    DungeonEntrance(String),
}

#[derive(Debug, Clone)]
pub enum PlayerAction {
    AddGold(i32),
    AddItem(Item),
    RemoveItemAt(usize),
    UseItem { index: usize },
    TakeDamage(i32),
    Heal(i32),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    None,
    ItemUsed,
    Died,
    ItemRemoved(Option<Item>),
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

    pub fn restore_stats(&mut self) {
        self.stats.current_hp = self.stats.max_hp;
        self.stats.current_mp = self.stats.max_mp;
    }

    pub fn has_item(&self, item_id: &str) -> bool {
        self.inventory.iter().any(|i| i.id == item_id)
    }

    pub fn can_use_skill(&self, cooldowns: &[u32; 3], slot: usize, mp_cost: i32) -> bool {
        slot < 3 && cooldowns[slot] == 0 && self.stats.current_mp >= mp_cost
    }

    pub fn apply(&mut self, action: PlayerAction) -> PlayerEvent {
        match action {
            PlayerAction::AddGold(amount) => {
                self.stats.gold = (self.stats.gold + amount).max(0);
                PlayerEvent::None
            }
            PlayerAction::AddItem(item) => {
                self.inventory.push(item);
                PlayerEvent::None
            }
            PlayerAction::RemoveItemAt(index) => {
                PlayerEvent::ItemRemoved(self.remove_item_at(index))
            }
            PlayerAction::UseItem { index } => {
                if self.use_item(index) {
                    PlayerEvent::ItemUsed
                } else {
                    PlayerEvent::None
                }
            }
            PlayerAction::TakeDamage(amount) => {
                self.stats.take_damage(amount);
                if self.stats.is_dead() {
                    PlayerEvent::Died
                } else {
                    PlayerEvent::None
                }
            }
            PlayerAction::Heal(amount) => {
                self.stats.heal(amount);
                PlayerEvent::None
            }
        }
    }

    pub fn apply_kill_reward(&mut self, reward: &KillReward) {
        self.stats.add_exp(reward.exp);
        self.stats.gold = (self.stats.gold + reward.gold).max(0);
    }

    fn use_item(&mut self, index: usize) -> bool {
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

    fn remove_item_at(&mut self, index: usize) -> Option<Item> {
        if index >= self.inventory.len() {
            return None;
        }

        let item = self.inventory.remove(index);
        self.fix_equipped_indices(index);
        Some(item)
    }

    fn fix_equipped_indices(&mut self, removed_index: usize) {
        if let Some(ref mut index) = self.equipped_weapon {
            if *index > removed_index {
                *index -= 1;
            } else if *index == removed_index {
                self.equipped_weapon = None;
            }
        }
        if let Some(ref mut index) = self.equipped_armor {
            if *index > removed_index {
                *index -= 1;
            } else if *index == removed_index {
                self.equipped_armor = None;
            }
        }
        if let Some(ref mut index) = self.equipped_accessory {
            if *index > removed_index {
                *index -= 1;
            } else if *index == removed_index {
                self.equipped_accessory = None;
            }
        }
    }
}

impl CharacterState {
    pub fn apply_event(&mut self, data: &GameData, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Session(session_event) => match session_event {
                SessionEvent::Create => {}
                SessionEvent::SetPlayerName(name) => {
                    self.name = name.clone();
                }
                SessionEvent::SetPlayerStats(stats) => {
                    self.stats = stats.clone();
                }
                SessionEvent::SetPlayerMap(map_id) => {
                    self.current_map_id = map_id.clone();
                }
                SessionEvent::SetPlayerPosition { x, y } => {
                    self.x = *x;
                    self.y = *y;
                }
                SessionEvent::SetPlayerFacing(facing) => {
                    self.facing = *facing;
                }
                SessionEvent::AddPlayerItem(item) => {
                    self.inventory.push(item.clone());
                }
                SessionEvent::SetEquippedWeapon(index) => {
                    self.equipped_weapon = *index;
                }
                SessionEvent::SetEquippedArmor(index) => {
                    self.equipped_armor = *index;
                }
                SessionEvent::SetEquippedAccessory(index) => {
                    self.equipped_accessory = *index;
                }
                SessionEvent::AddQuestProgress(_)
                | SessionEvent::AddOpenedTreasure { .. }
                | SessionEvent::SetSkillCooldowns(_)
                | SessionEvent::SetMpRegenTimer(_)
                | SessionEvent::ResetMovement
                | SessionEvent::ResetCombat
                | SessionEvent::SpawnCurrentMapEnemies => {}
            },
            GameEvent::ApplyDialogAction(action) => match action {
                crate::data::DialogAction::GiveQuest(_)
                | crate::data::DialogAction::CompleteQuest(_) => {}
                crate::data::DialogAction::GiveItem(id) => {
                    if let Some(item) = data.find_item(id).cloned() {
                        let _ = self.apply(PlayerAction::AddItem(item));
                    }
                }
                crate::data::DialogAction::TakeItem(id) => {
                    if let Some(index) = self.inventory.iter().position(|item| item.id == *id) {
                        let _ = self.apply(PlayerAction::RemoveItemAt(index));
                    }
                }
                crate::data::DialogAction::GiveGold(amount) => {
                    let _ = self.apply(PlayerAction::AddGold(*amount));
                }
                crate::data::DialogAction::TakeGold(amount) => {
                    let _ = self.apply(PlayerAction::AddGold(-*amount));
                }
                crate::data::DialogAction::OpenShop(_) => {}
                crate::data::DialogAction::Heal => {
                    self.stats.current_hp = self.stats.max_hp;
                    self.stats.current_mp = self.stats.max_mp;
                }
            },
            GameEvent::Inventory(crate::game::InventoryEvent::UseSelected(index)) => {
                let _ = self.apply(PlayerAction::UseItem { index: *index });
            }
            GameEvent::Shop(crate::game::ShopEvent::BuyItem(item)) => {
                let _ = self.apply(PlayerAction::AddGold(-item.price));
                let _ = self.apply(PlayerAction::AddItem(item.clone()));
            }
            GameEvent::Shop(crate::game::ShopEvent::SellSelected(index)) => {
                if let PlayerEvent::ItemRemoved(Some(item)) =
                    self.apply(PlayerAction::RemoveItemAt(*index))
                {
                    let _ = self.apply(PlayerAction::AddGold(item.price / 2));
                }
            }
            _ => {}
        }
        Ok(())
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
        player.inventory.push(make_potion());
        assert!(player.has_item("potion"));
        assert!(!player.has_item("ether"));
    }

    #[test]
    fn equip_weapon_via_use_item() {
        let mut player = CharacterState::new(String::from("H"), "v");
        let _ = player.apply(PlayerAction::AddItem(make_item("sword", ItemKind::Weapon)));
        assert!(player.use_item(0));
        assert_eq!(player.equipped_weapon, Some(0));
    }

    #[test]
    fn equip_armor_via_use_item() {
        let mut player = CharacterState::new(String::from("H"), "v");
        let _ = player.apply(PlayerAction::AddItem(make_item("armor", ItemKind::Armor)));
        assert!(player.use_item(0));
        assert_eq!(player.equipped_armor, Some(0));
    }

    #[test]
    fn use_consumable_heals_and_removes() {
        let mut player = CharacterState::new(String::from("H"), "v");
        player.stats.current_hp = 20;
        let _ = player.apply(PlayerAction::AddItem(make_potion()));
        assert!(player.use_item(0));
        assert_eq!(player.stats.current_hp, 50);
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn fix_equipped_indices_on_remove() {
        let mut player = CharacterState::new(String::from("H"), "v");
        let _ = player.apply(PlayerAction::AddItem(make_potion()));
        let _ = player.apply(PlayerAction::AddItem(make_item("sword", ItemKind::Weapon)));
        let _ = player.apply(PlayerAction::AddItem(make_item("armor", ItemKind::Armor)));
        player.equipped_weapon = Some(1);
        player.equipped_armor = Some(2);

        let _ = player.use_item(0);
        assert_eq!(player.equipped_weapon, Some(0));
        assert_eq!(player.equipped_armor, Some(1));
    }

    #[test]
    fn fix_equipped_clears_on_exact_removal() {
        let mut player = CharacterState::new(String::from("H"), "v");
        let _ = player.apply(PlayerAction::AddItem(make_item("sword", ItemKind::Weapon)));
        player.equipped_weapon = Some(0);

        player.inventory.remove(0);
        player.fix_equipped_indices(0);
        assert_eq!(player.equipped_weapon, None);
    }

    #[test]
    fn use_item_out_of_bounds() {
        let mut player = CharacterState::new(String::from("H"), "v");
        assert!(!player.use_item(0));
        assert!(!player.use_item(99));
    }

    #[test]
    fn remove_item_at_returns_item() {
        let mut player = CharacterState::new(String::from("H"), "v");
        let _ = player.apply(PlayerAction::AddItem(make_potion()));
        let event = player.apply(PlayerAction::RemoveItemAt(0));
        let PlayerEvent::ItemRemoved(removed) = event else {
            panic!("expected ItemRemoved event");
        };
        assert_eq!(
            removed.as_ref().map(|item| item.id.as_str()),
            Some("potion")
        );
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn remove_item_at_out_of_bounds() {
        let mut player = CharacterState::new(String::from("H"), "v");
        let event = player.apply(PlayerAction::RemoveItemAt(0));
        let PlayerEvent::ItemRemoved(removed) = event else {
            panic!("expected ItemRemoved event");
        };
        assert!(removed.is_none());
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

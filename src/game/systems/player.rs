use crate::data::{Item, ItemKind};
use crate::game::PlayerState;

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

pub fn apply(player: &mut PlayerState, action: PlayerAction) -> PlayerEvent {
    match action {
        PlayerAction::AddGold(amount) => {
            player.stats.gold = (player.stats.gold + amount).max(0);
            PlayerEvent::None
        }
        PlayerAction::AddItem(item) => {
            add_item(player, item);
            PlayerEvent::None
        }
        PlayerAction::RemoveItemAt(index) => {
            PlayerEvent::ItemRemoved(remove_item_at(player, index))
        }
        PlayerAction::UseItem { index } => {
            if use_item(player, index) {
                PlayerEvent::ItemUsed
            } else {
                PlayerEvent::None
            }
        }
        PlayerAction::TakeDamage(amount) => {
            player.stats.take_damage(amount);
            if player.stats.is_dead() {
                PlayerEvent::Died
            } else {
                PlayerEvent::None
            }
        }
        PlayerAction::Heal(amount) => {
            player.stats.heal(amount);
            PlayerEvent::None
        }
    }
}

pub fn can_use_skill(
    player: &PlayerState,
    cooldowns: &[u32; 3],
    slot: usize,
    mp_cost: i32,
) -> bool {
    slot < 3 && cooldowns[slot] == 0 && player.stats.current_mp >= mp_cost
}

fn use_item(player: &mut PlayerState, index: usize) -> bool {
    if index >= player.inventory.len() {
        return false;
    }

    let item = &player.inventory[index];
    match item.kind {
        ItemKind::Consumable => {
            let heal = item.hp_restore();
            player.stats.heal(heal);
            player.inventory.remove(index);
            fix_equipped_indices(player, index);
            true
        }
        ItemKind::Weapon => {
            player.equipped_weapon = Some(index);
            true
        }
        ItemKind::Armor => {
            player.equipped_armor = Some(index);
            true
        }
        ItemKind::Accessory => {
            player.equipped_accessory = Some(index);
            true
        }
    }
}

fn add_item(player: &mut PlayerState, item: Item) {
    player.inventory.push(item);
}

fn remove_item_at(player: &mut PlayerState, index: usize) -> Option<Item> {
    if index >= player.inventory.len() {
        return None;
    }

    let item = player.inventory.remove(index);
    fix_equipped_indices(player, index);
    Some(item)
}

fn fix_equipped_indices(player: &mut PlayerState, removed_index: usize) {
    if let Some(ref mut index) = player.equipped_weapon {
        if *index > removed_index {
            *index -= 1;
        } else if *index == removed_index {
            player.equipped_weapon = None;
        }
    }
    if let Some(ref mut index) = player.equipped_armor {
        if *index > removed_index {
            *index -= 1;
        } else if *index == removed_index {
            player.equipped_armor = None;
        }
    }
    if let Some(ref mut index) = player.equipped_accessory {
        if *index > removed_index {
            *index -= 1;
        } else if *index == removed_index {
            player.equipped_accessory = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;
    use crate::data::ItemKind;

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
    fn equip_weapon_via_use_item() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = apply(
            &mut player,
            PlayerAction::AddItem(make_item("sword", ItemKind::Weapon)),
        );
        assert!(use_item(&mut player, 0));
        assert_eq!(player.equipped_weapon, Some(0));
    }

    #[test]
    fn equip_armor_via_use_item() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = apply(
            &mut player,
            PlayerAction::AddItem(make_item("armor", ItemKind::Armor)),
        );
        assert!(use_item(&mut player, 0));
        assert_eq!(player.equipped_armor, Some(0));
    }

    #[test]
    fn use_consumable_heals_and_removes() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.stats.current_hp = 20;
        let _ = apply(&mut player, PlayerAction::AddItem(make_potion()));
        assert!(use_item(&mut player, 0));
        assert_eq!(player.stats.current_hp, 50);
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn fix_equipped_indices_on_remove() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = apply(&mut player, PlayerAction::AddItem(make_potion()));
        let _ = apply(
            &mut player,
            PlayerAction::AddItem(make_item("sword", ItemKind::Weapon)),
        );
        let _ = apply(
            &mut player,
            PlayerAction::AddItem(make_item("armor", ItemKind::Armor)),
        );
        player.equipped_weapon = Some(1);
        player.equipped_armor = Some(2);

        let _ = use_item(&mut player, 0);
        assert_eq!(player.equipped_weapon, Some(0));
        assert_eq!(player.equipped_armor, Some(1));
    }

    #[test]
    fn fix_equipped_clears_on_exact_removal() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = apply(
            &mut player,
            PlayerAction::AddItem(make_item("sword", ItemKind::Weapon)),
        );
        player.equipped_weapon = Some(0);

        player.inventory.remove(0);
        fix_equipped_indices(&mut player, 0);
        assert_eq!(player.equipped_weapon, None);
    }

    #[test]
    fn use_item_out_of_bounds() {
        let mut player = PlayerState::new(String::from("H"), "v");
        assert!(!use_item(&mut player, 0));
        assert!(!use_item(&mut player, 99));
    }

    #[test]
    fn remove_item_at_returns_item() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = apply(&mut player, PlayerAction::AddItem(make_potion()));
        let event = apply(&mut player, PlayerAction::RemoveItemAt(0));
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
        let mut player = PlayerState::new(String::from("H"), "v");
        let event = apply(&mut player, PlayerAction::RemoveItemAt(0));
        let PlayerEvent::ItemRemoved(removed) = event else {
            panic!("expected ItemRemoved event");
        };
        assert!(removed.is_none());
    }

    #[test]
    fn add_quest_no_duplicates() {
        let mut player = PlayerState::new(String::from("H"), "v");
        if !player.has_quest("q1") {
            player.quests.push(crate::data::QuestProgress {
                quest_id: String::from("q1"),
                current_count: 0,
                completed: false,
                rewarded: false,
            });
        }
        if !player.has_quest("q1") {
            player.quests.push(crate::data::QuestProgress {
                quest_id: String::from("q1"),
                current_count: 0,
                completed: false,
                rewarded: false,
            });
        }
        assert_eq!(player.quests.len(), 1);
    }

    #[test]
    fn treasure_tracking_no_duplicates() {
        let mut player = PlayerState::new(String::from("H"), "v");
        if !player.is_treasure_opened("map1", 3, 4) {
            player.opened_treasures.push((String::from("map1"), 3, 4));
        }
        if !player.is_treasure_opened("map1", 3, 4) {
            player.opened_treasures.push((String::from("map1"), 3, 4));
        }

        assert!(player.is_treasure_opened("map1", 3, 4));
        assert_eq!(player.opened_treasures.len(), 1);
    }

    #[test]
    fn mark_quest_rewarded_intent_marks_rewarded() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.quests.push(crate::data::QuestProgress {
            quest_id: String::from("q1"),
            current_count: 0,
            completed: false,
            rewarded: false,
        });
        if let Some(quest) = player
            .quests
            .iter_mut()
            .find(|quest| quest.quest_id == "q1")
        {
            quest.rewarded = true;
        }
        assert!(player.quests[0].rewarded);
    }

    #[test]
    fn skill_cooldowns() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let mut cooldowns = [0; 3];
        assert!(can_use_skill(&player, &cooldowns, 0, 10));

        cooldowns[0] = 30;
        player.stats.current_mp = 20;
        assert!(!can_use_skill(&player, &cooldowns, 0, 10));

        cooldowns[0] = 0;
        assert!(can_use_skill(&player, &cooldowns, 0, 10));
    }

    #[test]
    fn skill_insufficient_mp() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.stats.current_mp = 5;
        assert!(!can_use_skill(&player, &[0; 3], 0, 10));
    }
}

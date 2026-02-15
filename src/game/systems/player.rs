use alloc::string::String;

use crate::data::{Direction, Item, ItemKind, QuestProgress};
use crate::game::PlayerState;

#[derive(Debug, Clone)]
pub enum PlayerIntent {
    AddExp(i32),
    AddGold(i32),
    FullHeal,
    AddItem(Item),
    RemoveItem(String),
    RemoveItemAt(usize),
    EquipWeapon(usize),
    EquipArmor(usize),
    SetFacing { dx: i32, dy: i32 },
    MoveBy { dx: i32, dy: i32 },
    ChangeMap { map_id: String, x: usize, y: usize },
    SpawnAtMap { x: usize, y: usize },
    OpenTreasure { map_id: String, x: usize, y: usize },
    AddQuest(String),
    MarkQuestRewarded(String),
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

pub fn reduce(player: &mut PlayerState, intent: PlayerIntent) -> PlayerEvent {
    match intent {
        PlayerIntent::AddExp(exp) => {
            player.stats.add_exp(exp);
            PlayerEvent::None
        }
        PlayerIntent::AddGold(amount) => {
            player.stats.gold = (player.stats.gold + amount).max(0);
            PlayerEvent::None
        }
        PlayerIntent::FullHeal => {
            player.stats.current_hp = player.stats.max_hp;
            player.stats.current_mp = player.stats.max_mp;
            PlayerEvent::None
        }
        PlayerIntent::AddItem(item) => {
            add_item(player, item);
            PlayerEvent::None
        }
        PlayerIntent::RemoveItem(id) => {
            let _ = remove_item(player, &id);
            PlayerEvent::None
        }
        PlayerIntent::RemoveItemAt(index) => {
            PlayerEvent::ItemRemoved(remove_item_at(player, index))
        }
        PlayerIntent::EquipWeapon(idx) => {
            player.equipped_weapon = Some(idx);
            PlayerEvent::None
        }
        PlayerIntent::EquipArmor(idx) => {
            player.equipped_armor = Some(idx);
            PlayerEvent::None
        }
        PlayerIntent::SetFacing { dx, dy } => {
            set_facing(player, dx, dy);
            PlayerEvent::None
        }
        PlayerIntent::MoveBy { dx, dy } => {
            move_by(player, dx, dy);
            PlayerEvent::None
        }
        PlayerIntent::ChangeMap { map_id, x, y } => {
            player.current_map_id = map_id;
            player.x = x;
            player.y = y;
            PlayerEvent::None
        }
        PlayerIntent::SpawnAtMap { x, y } => {
            player.x = x;
            player.y = y;
            PlayerEvent::None
        }
        PlayerIntent::OpenTreasure { map_id, x, y } => {
            open_treasure(player, &map_id, x, y);
            PlayerEvent::None
        }
        PlayerIntent::AddQuest(id) => {
            add_quest(player, &id);
            PlayerEvent::None
        }
        PlayerIntent::MarkQuestRewarded(id) => {
            mark_quest_rewarded(player, &id);
            PlayerEvent::None
        }
        PlayerIntent::UseItem { index } => {
            if use_item(player, index) {
                PlayerEvent::ItemUsed
            } else {
                PlayerEvent::None
            }
        }
        PlayerIntent::TakeDamage(amount) => {
            player.stats.take_damage(amount);
            if player.stats.is_dead() {
                PlayerEvent::Died
            } else {
                PlayerEvent::None
            }
        }
        PlayerIntent::Heal(amount) => {
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

fn remove_item(player: &mut PlayerState, item_id: &str) -> bool {
    if let Some(index) = player.inventory.iter().position(|item| item.id == item_id) {
        player.inventory.remove(index);
        fix_equipped_indices(player, index);
        true
    } else {
        false
    }
}

fn remove_item_at(player: &mut PlayerState, index: usize) -> Option<Item> {
    if index >= player.inventory.len() {
        return None;
    }

    let item = player.inventory.remove(index);
    fix_equipped_indices(player, index);
    Some(item)
}

fn set_facing(player: &mut PlayerState, dx: i32, dy: i32) {
    player.facing = match (dx, dy) {
        (0, -1) => Direction::Up,
        (0, 1) => Direction::Down,
        (-1, 0) => Direction::Left,
        (1, 0) => Direction::Right,
        _ => player.facing,
    };
}

fn move_by(player: &mut PlayerState, dx: i32, dy: i32) {
    if let Some(new_x) = player.x.checked_add_signed(dx as isize) {
        player.x = new_x;
    }
    if let Some(new_y) = player.y.checked_add_signed(dy as isize) {
        player.y = new_y;
    }
    set_facing(player, dx, dy);
}

fn open_treasure(player: &mut PlayerState, map_id: &str, x: usize, y: usize) {
    if !player.is_treasure_opened(map_id, x, y) {
        player.opened_treasures.push((map_id.into(), x, y));
    }
}

fn add_quest(player: &mut PlayerState, quest_id: &str) {
    if !player.has_quest(quest_id) {
        player.quests.push(QuestProgress {
            quest_id: quest_id.into(),
            current_count: 0,
            completed: false,
            rewarded: false,
        });
    }
}

fn mark_quest_rewarded(player: &mut PlayerState, quest_id: &str) {
    if let Some(quest) = player
        .quests
        .iter_mut()
        .find(|quest| quest.quest_id == quest_id)
    {
        quest.rewarded = true;
    }
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
        let _ = reduce(
            &mut player,
            PlayerIntent::AddItem(make_item("sword", ItemKind::Weapon)),
        );
        assert!(use_item(&mut player, 0));
        assert_eq!(player.equipped_weapon, Some(0));
    }

    #[test]
    fn equip_armor_via_use_item() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = reduce(
            &mut player,
            PlayerIntent::AddItem(make_item("armor", ItemKind::Armor)),
        );
        assert!(use_item(&mut player, 0));
        assert_eq!(player.equipped_armor, Some(0));
    }

    #[test]
    fn use_consumable_heals_and_removes() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.stats.current_hp = 20;
        let _ = reduce(&mut player, PlayerIntent::AddItem(make_potion()));
        assert!(use_item(&mut player, 0));
        assert_eq!(player.stats.current_hp, 50);
        assert!(player.inventory.is_empty());
    }

    #[test]
    fn fix_equipped_indices_on_remove() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = reduce(&mut player, PlayerIntent::AddItem(make_potion()));
        let _ = reduce(
            &mut player,
            PlayerIntent::AddItem(make_item("sword", ItemKind::Weapon)),
        );
        let _ = reduce(
            &mut player,
            PlayerIntent::AddItem(make_item("armor", ItemKind::Armor)),
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
        let _ = reduce(
            &mut player,
            PlayerIntent::AddItem(make_item("sword", ItemKind::Weapon)),
        );
        player.equipped_weapon = Some(0);

        let _ = reduce(&mut player, PlayerIntent::RemoveItem(String::from("sword")));
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
        let _ = reduce(&mut player, PlayerIntent::AddItem(make_potion()));
        let event = reduce(&mut player, PlayerIntent::RemoveItemAt(0));
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
        let event = reduce(&mut player, PlayerIntent::RemoveItemAt(0));
        let PlayerEvent::ItemRemoved(removed) = event else {
            panic!("expected ItemRemoved event");
        };
        assert!(removed.is_none());
    }

    #[test]
    fn add_quest_no_duplicates() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = reduce(&mut player, PlayerIntent::AddQuest(String::from("q1")));
        let _ = reduce(&mut player, PlayerIntent::AddQuest(String::from("q1")));
        assert_eq!(player.quests.len(), 1);
    }

    #[test]
    fn treasure_tracking_no_duplicates() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = reduce(
            &mut player,
            PlayerIntent::OpenTreasure {
                map_id: String::from("map1"),
                x: 3,
                y: 4,
            },
        );
        let _ = reduce(
            &mut player,
            PlayerIntent::OpenTreasure {
                map_id: String::from("map1"),
                x: 3,
                y: 4,
            },
        );

        assert!(player.is_treasure_opened("map1", 3, 4));
        assert_eq!(player.opened_treasures.len(), 1);
    }

    #[test]
    fn mark_quest_rewarded_intent_marks_rewarded() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = reduce(&mut player, PlayerIntent::AddQuest(String::from("q1")));
        let _ = reduce(
            &mut player,
            PlayerIntent::MarkQuestRewarded(String::from("q1")),
        );
        assert!(player.quests[0].rewarded);
    }

    #[test]
    fn move_by_and_set_facing_intents_update_position_and_direction() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let _ = reduce(&mut player, PlayerIntent::SetFacing { dx: 1, dy: 0 });
        assert!(matches!(player.facing, Direction::Right));

        let _ = reduce(&mut player, PlayerIntent::MoveBy { dx: 0, dy: 1 });
        assert_eq!(player.x, 0);
        assert_eq!(player.y, 1);
        assert!(matches!(player.facing, Direction::Down));
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

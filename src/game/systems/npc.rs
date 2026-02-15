use crate::data::{Dialog, DialogAction, DialogCondition, Direction, NpcType};
use crate::game::{self, DialogState, GameData, GameState, PlayerIntent, PlayerState, ShopState};

#[derive(Debug)]
pub enum NpcIntent<'a> {
    Interact { facing: Direction },
    ProcessDialogAction { action: &'a DialogAction },
}

pub fn reduce(
    player: &mut PlayerState,
    data: &GameData,
    intent: NpcIntent<'_>,
) -> Option<GameState> {
    match intent {
        NpcIntent::Interact { facing } => try_interact(player, data, facing),
        NpcIntent::ProcessDialogAction { action } => process_action(player, data, action),
    }
}

pub fn try_interact(
    player: &mut PlayerState,
    data: &GameData,
    facing: Direction,
) -> Option<GameState> {
    let (target_x, target_y) = facing.apply(player.x, player.y);

    let npc = data.find_npc_at(&player.current_map_id, target_x, target_y)?;

    match npc.npc_type {
        NpcType::Healer => {
            let _ = game::player::reduce(player, PlayerIntent::FullHeal);

            if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
                let filtered = filter_lines(player, dialog);
                if !filtered.lines.is_empty() {
                    return Some(GameState::Dialog(DialogState::new(
                        npc.name.clone(),
                        &filtered,
                    )));
                }
            }
        }
        NpcType::ShopKeeper => {
            let shop = npc
                .shop_id
                .as_ref()
                .and_then(|sid| data.find_shop(sid))
                .or_else(|| data.shops.first())
                .cloned();

            if let Some(shop) = shop {
                let shop_items = data.get_shop_items(&shop);
                return Some(GameState::Shop(ShopState::new(shop, shop_items)));
            }
        }
        NpcType::QuestGiver | NpcType::Villager => {}
    }

    if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
        let filtered = filter_lines(player, dialog);
        if !filtered.lines.is_empty() {
            return Some(GameState::Dialog(DialogState::new(
                npc.name.clone(),
                &filtered,
            )));
        }
    }

    None
}

pub fn filter_lines(player: &PlayerState, dialog: &Dialog) -> Dialog {
    let filtered = dialog
        .lines
        .iter()
        .filter(|line| match &line.condition {
            None => true,
            Some(DialogCondition::HasQuest(id)) => player.has_quest(id),
            Some(DialogCondition::QuestComplete(id)) => player.is_quest_complete(id),
            Some(DialogCondition::HasItem(id)) => player.has_item(id),
            Some(DialogCondition::HasGold(amount)) => player.stats.gold >= *amount,
        })
        .cloned()
        .collect();

    Dialog {
        id: dialog.id.clone(),
        lines: filtered,
    }
}

pub fn process_action(
    player: &mut PlayerState,
    data: &GameData,
    action: &DialogAction,
) -> Option<GameState> {
    match action {
        DialogAction::GiveQuest(id) => {
            let _ = game::player::reduce(player, PlayerIntent::AddQuest(id.clone()));
        }
        DialogAction::CompleteQuest(id) => {
            let can_reward = player
                .quests
                .iter()
                .any(|q| q.quest_id == *id && q.completed && !q.rewarded);

            if can_reward && let Some(quest) = data.find_quest(id) {
                let _ = game::player::reduce(player, PlayerIntent::AddExp(quest.reward_exp));
                let _ = game::player::reduce(player, PlayerIntent::AddGold(quest.reward_gold));

                if let Some(item_id) = &quest.reward_item
                    && let Some(item) = data.find_item(item_id).cloned()
                {
                    let _ = game::player::reduce(player, PlayerIntent::AddItem(item));
                }

                let _ = game::player::reduce(player, PlayerIntent::MarkQuestRewarded(id.clone()));
            }
        }
        DialogAction::GiveItem(id) => {
            if let Some(item) = data.find_item(id).cloned() {
                let _ = game::player::reduce(player, PlayerIntent::AddItem(item));
            }
        }
        DialogAction::TakeItem(id) => {
            let _ = game::player::reduce(player, PlayerIntent::RemoveItem(id.clone()));
        }
        DialogAction::GiveGold(amount) => {
            let _ = game::player::reduce(player, PlayerIntent::AddGold(*amount));
        }
        DialogAction::TakeGold(amount) => {
            let _ = game::player::reduce(player, PlayerIntent::AddGold(-*amount));
        }
        DialogAction::OpenShop(id) => {
            if let Some(shop) = data.find_shop(id).cloned() {
                let shop_items = data.get_shop_items(&shop);
                return Some(GameState::Shop(ShopState::new(shop, shop_items)));
            }
        }
        DialogAction::Heal => {
            let _ = game::player::reduce(player, PlayerIntent::FullHeal);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Item, ItemKind, Npc, QuestProgress, Shop};

    fn make_npc(npc_type: NpcType) -> Npc {
        Npc {
            id: String::from("npc1"),
            name: String::from("NPC"),
            map_id: String::from("v"),
            x: 1,
            y: 0,
            npc_type,
            dialog_id: String::from("d1"),
            shop_id: Some(String::from("s1")),
        }
    }

    fn make_dialog(lines: Vec<&str>) -> Dialog {
        let mut raw = String::from("@DIALOG:d1\n");
        for line in lines {
            raw.push_str(line);
            raw.push('\n');
        }
        raw.push_str("@END\n");

        let Ok(dialogs) = crate::data::parse_dialogs(&raw) else {
            panic!("failed to parse dialog");
        };
        let Some(dialog) = dialogs.into_iter().next() else {
            panic!("dialog parse returned empty list");
        };
        dialog
    }

    fn make_game_data_with_npc(npc: Npc, dialog: Dialog) -> GameData {
        GameData {
            npcs: vec![npc],
            dialogs: vec![dialog],
            ..GameData::default()
        }
    }

    fn make_item(id: &str) -> Item {
        Item {
            id: String::from(id),
            name: String::from(id),
            kind: ItemKind::Consumable,
            param1: 10,
            param2: 0,
            param3: 0,
            price: 20,
        }
    }

    fn make_shop(id: &str, items: Vec<String>) -> Shop {
        Shop {
            id: String::from(id),
            name: String::from("General Store"),
            items,
        }
    }

    #[test]
    fn try_interact_returns_none_when_no_npc_at_facing_position() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let data = GameData::default();

        let next_state = try_interact(&mut player, &data, Direction::Right);

        assert!(next_state.is_none());
    }

    #[test]
    fn try_interact_with_villager_returns_dialog_state() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let npc = make_npc(NpcType::Villager);
        let dialog = make_dialog(vec!["Hello"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&mut player, &data, Direction::Right);

        let Some(GameState::Dialog(dialog_state)) = next_state else {
            panic!("expected dialog state");
        };
        assert_eq!(dialog_state.npc_name, "NPC");
        assert_eq!(dialog_state.dialog_id, "d1");
    }

    #[test]
    fn try_interact_with_healer_fully_heals_and_returns_dialog_state() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.stats.current_hp = 7;
        player.stats.current_mp = 3;

        let npc = make_npc(NpcType::Healer);
        let dialog = make_dialog(vec!["Be healed"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&mut player, &data, Direction::Right);

        assert_eq!(player.stats.current_hp, player.stats.max_hp);
        assert_eq!(player.stats.current_mp, player.stats.max_mp);
        assert!(matches!(next_state, Some(GameState::Dialog(_))));
    }

    #[test]
    fn try_interact_with_shopkeeper_returns_shop_state() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let npc = make_npc(NpcType::ShopKeeper);
        let dialog = make_dialog(vec!["Welcome"]);
        let mut data = make_game_data_with_npc(npc, dialog);
        data.shops = vec![make_shop("s1", Vec::new())];

        let next_state = try_interact(&mut player, &data, Direction::Right);

        let Some(GameState::Shop(shop_state)) = next_state else {
            panic!("expected shop state");
        };
        assert_eq!(shop_state.shop.id, "s1");
    }

    #[test]
    fn filter_lines_without_conditions_keeps_all_lines() {
        let player = PlayerState::new(String::from("H"), "v");
        let dialog = make_dialog(vec!["A", "B", "C"]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.lines.len(), 3);
    }

    #[test]
    fn filter_lines_has_quest_keeps_only_matching_lines() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 0,
            completed: false,
            rewarded: false,
        });
        let dialog = make_dialog(vec!["HAS_QUEST=q1:talk:q1", "HAS_QUEST=q2:talk:q2"]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.lines.len(), 1);
        assert_eq!(filtered.lines[0].text, "q1");
    }

    #[test]
    fn filter_lines_quest_complete_condition_filters_correctly() {
        let mut player = PlayerState::new(String::from("H"), "v");
        player.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 1,
            completed: true,
            rewarded: false,
        });
        let dialog = make_dialog(vec![
            "QUEST_DONE=q1:talk:done",
            "QUEST_DONE=q2:talk:not done",
        ]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.lines.len(), 1);
        assert_eq!(filtered.lines[0].text, "done");
    }

    #[test]
    fn filter_lines_has_gold_condition_filters_correctly() {
        let player = PlayerState::new(String::from("H"), "v");
        let dialog = make_dialog(vec![
            "HAS_GOLD=10:talk:cheap",
            "HAS_GOLD=100:talk:expensive",
        ]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.lines.len(), 1);
        assert_eq!(filtered.lines[0].text, "cheap");
    }

    #[test]
    fn process_action_give_quest_adds_quest() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let data = GameData::default();

        let next_state = process_action(
            &mut player,
            &data,
            &DialogAction::GiveQuest(String::from("q1")),
        );

        assert!(next_state.is_none());
        assert!(player.has_quest("q1"));
    }

    #[test]
    fn process_action_give_item_adds_item_when_present_in_data() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let data = GameData {
            items: vec![make_item("potion")],
            ..GameData::default()
        };

        let next_state = process_action(
            &mut player,
            &data,
            &DialogAction::GiveItem(String::from("potion")),
        );

        assert!(next_state.is_none());
        assert!(player.has_item("potion"));
    }

    #[test]
    fn process_action_give_gold_and_take_gold_modify_gold() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let data = GameData::default();
        let starting_gold = player.stats.gold;

        let _ = process_action(&mut player, &data, &DialogAction::GiveGold(25));
        assert_eq!(player.stats.gold, starting_gold + 25);

        let _ = process_action(&mut player, &data, &DialogAction::TakeGold(15));
        assert_eq!(player.stats.gold, starting_gold + 10);
    }

    #[test]
    fn process_action_heal_fully_heals_player() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let data = GameData::default();
        player.stats.current_hp = 2;
        player.stats.current_mp = 1;

        let next_state = process_action(&mut player, &data, &DialogAction::Heal);

        assert!(next_state.is_none());
        assert_eq!(player.stats.current_hp, player.stats.max_hp);
        assert_eq!(player.stats.current_mp, player.stats.max_mp);
    }

    #[test]
    fn process_action_open_shop_returns_shop_state() {
        let mut player = PlayerState::new(String::from("H"), "v");
        let data = GameData {
            items: vec![make_item("potion")],
            shops: vec![make_shop("s1", vec![String::from("potion")])],
            ..GameData::default()
        };

        let next_state = process_action(
            &mut player,
            &data,
            &DialogAction::OpenShop(String::from("s1")),
        );

        let Some(GameState::Shop(shop_state)) = next_state else {
            panic!("expected shop state");
        };
        assert_eq!(shop_state.shop.id, "s1");
        assert_eq!(shop_state.items.len(), 1);
        assert_eq!(shop_state.items[0].id, "potion");
    }
}

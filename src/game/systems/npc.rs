use crate::data::{Dialog, DialogCondition, Direction, NpcType};
use crate::game::{DialogState, GameData, PlayerState, ShopState};

#[derive(Debug)]
pub enum NpcEvent {
    OpenDialog {
        dialog_state: DialogState,
        restore: bool,
    },
    OpenShop(ShopState),
    RestoreStats,
}

#[derive(Debug)]
pub enum NpcIntent {
    Interact { facing: Direction },
}

pub fn reduce(player: &PlayerState, data: &GameData, intent: NpcIntent) -> Option<NpcEvent> {
    match intent {
        NpcIntent::Interact { facing } => try_interact(player, data, facing),
    }
}

fn try_interact(player: &PlayerState, data: &GameData, facing: Direction) -> Option<NpcEvent> {
    let (target_x, target_y) = facing.apply(player.x, player.y);

    let npc = data.find_npc_at(&player.current_map_id, target_x, target_y)?;

    match npc.npc_type {
        NpcType::Healer => {
            if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
                let filtered = filter_lines(player, dialog);
                if !filtered.lines.is_empty() {
                    return Some(NpcEvent::OpenDialog {
                        dialog_state: DialogState::new(npc.name.clone(), &filtered),
                        restore: true,
                    });
                }
            }

            return Some(NpcEvent::RestoreStats);
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
                return Some(NpcEvent::OpenShop(ShopState::new(shop, shop_items)));
            }
        }
        NpcType::QuestGiver | NpcType::Villager => {}
    }

    if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
        let filtered = filter_lines(player, dialog);
        if !filtered.lines.is_empty() {
            return Some(NpcEvent::OpenDialog {
                dialog_state: DialogState::new(npc.name.clone(), &filtered),
                restore: false,
            });
        }
    }

    None
}

fn filter_lines(player: &PlayerState, dialog: &Dialog) -> Dialog {
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

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Npc, QuestProgress, Shop};

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

    fn make_shop(id: &str, items: Vec<String>) -> Shop {
        Shop {
            id: String::from(id),
            name: String::from("General Store"),
            items,
        }
    }

    #[test]
    fn try_interact_returns_none_when_no_npc_at_facing_position() {
        let player = PlayerState::new(String::from("H"), "v");
        let data = GameData::default();

        let next_state = try_interact(&player, &data, Direction::Right);

        assert!(next_state.is_none());
    }

    #[test]
    fn try_interact_with_villager_returns_dialog_state() {
        let player = PlayerState::new(String::from("H"), "v");
        let npc = make_npc(NpcType::Villager);
        let dialog = make_dialog(vec!["Hello"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&player, &data, Direction::Right);

        let Some(NpcEvent::OpenDialog {
            dialog_state,
            restore,
        }) = next_state
        else {
            panic!("expected dialog state");
        };
        assert!(!restore);
        assert_eq!(dialog_state.npc_name, "NPC");
        assert_eq!(dialog_state.dialog_id, "d1");
    }

    #[test]
    fn try_interact_with_healer_requests_restore_and_returns_dialog_state() {
        let player = PlayerState::new(String::from("H"), "v");
        let before_hp = player.stats.current_hp;
        let before_mp = player.stats.current_mp;

        let npc = make_npc(NpcType::Healer);
        let dialog = make_dialog(vec!["Be healed"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&player, &data, Direction::Right);

        assert_eq!(player.stats.current_hp, before_hp);
        assert_eq!(player.stats.current_mp, before_mp);
        assert!(matches!(
            next_state,
            Some(NpcEvent::OpenDialog { restore: true, .. })
        ));
    }

    #[test]
    fn try_interact_with_healer_without_dialog_still_requests_restore() {
        let mut player = PlayerState::new(String::from("H"), "v");

        player.stats.current_hp = 7;
        player.stats.current_mp = 3;
        let before_hp = player.stats.current_hp;
        let before_mp = player.stats.current_mp;

        let npc = make_npc(NpcType::Healer);
        let dialog = make_dialog(Vec::new());
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&player, &data, Direction::Right);

        assert_eq!(player.stats.current_hp, before_hp);
        assert_eq!(player.stats.current_mp, before_mp);
        assert!(matches!(next_state, Some(NpcEvent::RestoreStats)));
    }

    #[test]
    fn try_interact_with_shopkeeper_returns_shop_state() {
        let player = PlayerState::new(String::from("H"), "v");
        let npc = make_npc(NpcType::ShopKeeper);
        let dialog = make_dialog(vec!["Welcome"]);
        let mut data = make_game_data_with_npc(npc, dialog);
        data.shops = vec![make_shop("s1", Vec::new())];

        let next_state = try_interact(&player, &data, Direction::Right);

        let Some(NpcEvent::OpenShop(shop_state)) = next_state else {
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
}

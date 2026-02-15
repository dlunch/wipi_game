use wipi::event::KeyCode;

use crate::data::DialogAction;
use crate::game::{DialogState, GameData, PlayerState, ShopState};

#[derive(Debug, Clone, Copy)]
pub enum DialogIntent {
    Confirm,
    Back,
}

impl DialogIntent {
    pub fn intent_for_key(key: KeyCode) -> Option<DialogIntent> {
        match key {
            KeyCode::Ok => Some(DialogIntent::Confirm),
            KeyCode::Back => Some(DialogIntent::Back),
            _ => None,
        }
    }
}

pub enum DialogEvent {
    None,
    Transition(DialogTransition),
    Action(DialogAction, DialogTransition),
}

pub enum DialogTransition {
    Set(DialogState),
    CloseToExplore,
}

pub fn apply_action(
    player: &mut PlayerState,
    data: &GameData,
    action: &DialogAction,
) -> Option<ShopState> {
    match action {
        DialogAction::GiveQuest(id) => {
            if !player.quests.iter().any(|q| q.quest_id == *id) {
                player.quests.push(crate::data::QuestProgress {
                    quest_id: id.clone(),
                    current_count: 0,
                    completed: false,
                    rewarded: false,
                });
            }
        }
        DialogAction::CompleteQuest(id) => {
            let can_reward = player
                .quests
                .iter()
                .any(|q| q.quest_id == *id && q.completed && !q.rewarded);

            if can_reward && let Some(quest) = data.find_quest(id) {
                player.stats.add_exp(quest.reward_exp);
                player.stats.gold = (player.stats.gold + quest.reward_gold).max(0);

                if let Some(item_id) = &quest.reward_item
                    && let Some(item) = data.find_item(item_id).cloned()
                {
                    player.inventory.push(item);
                }

                if let Some(progress) = player.quests.iter_mut().find(|q| q.quest_id == *id) {
                    progress.rewarded = true;
                }
            }
        }
        DialogAction::GiveItem(id) => {
            if let Some(item) = data.find_item(id).cloned() {
                player.inventory.push(item);
            }
        }
        DialogAction::TakeItem(id) => {
            if let Some(index) = player.inventory.iter().position(|item| item.id == *id) {
                player.inventory.remove(index);
            }
        }
        DialogAction::GiveGold(amount) => {
            player.stats.gold = (player.stats.gold + *amount).max(0);
        }
        DialogAction::TakeGold(amount) => {
            player.stats.gold = (player.stats.gold - *amount).max(0);
        }
        DialogAction::OpenShop(id) => {
            if let Some(shop) = data.find_shop(id).cloned() {
                let shop_items = data.get_shop_items(&shop);
                return Some(ShopState::new(shop, shop_items));
            }
        }
        DialogAction::Heal => {
            player.stats.current_hp = player.stats.max_hp;
            player.stats.current_mp = player.stats.max_mp;
        }
    }

    None
}

pub fn reduce(
    dialog_state: Option<&DialogState>,
    data: &GameData,
    intent: DialogIntent,
) -> DialogEvent {
    match intent {
        DialogIntent::Confirm => {
            if let Some(dialog_state_ref) = dialog_state
                && let Some(dialog) = data.find_dialog(&dialog_state_ref.dialog_id)
            {
                let transition = if dialog_state_ref.current_line + 1 < dialog.lines.len() {
                    let mut next = DialogState::new(dialog_state_ref.npc_name.clone(), dialog);
                    next.current_line = dialog_state_ref.current_line + 1;
                    DialogTransition::Set(next)
                } else {
                    DialogTransition::CloseToExplore
                };

                if let Some(action) = dialog
                    .lines
                    .get(dialog_state_ref.current_line)
                    .and_then(|line| line.action.as_ref())
                    .cloned()
                {
                    return DialogEvent::Action(action, transition);
                }

                return DialogEvent::Transition(transition);
            }
        }
        DialogIntent::Back => return DialogEvent::Transition(DialogTransition::CloseToExplore),
    }

    DialogEvent::None
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;

    use wipi::event::KeyCode;

    use super::{DialogEvent, DialogIntent, DialogTransition, reduce};
    use crate::data::{Dialog, DialogAction, Shop, parse_dialogs};
    use crate::game::{DialogState, GameData};

    fn make_dialog(id: &str, lines_count: usize) -> Dialog {
        let mut raw = alloc::format!("@DIALOG:{id}\n");
        for _ in 0..lines_count {
            raw.push_str("line\n");
        }
        raw.push_str("@END");

        let parsed = parse_dialogs(&raw);
        let mut dialogs = match parsed {
            Ok(dialogs) => dialogs,
            Err(_) => panic!("failed to build test dialog"),
        };

        if dialogs.len() == 1 {
            dialogs.remove(0)
        } else {
            panic!("expected exactly one dialog");
        }
    }

    fn make_dialog_with_action(id: &str, action: DialogAction) -> Dialog {
        let action_token = match action {
            DialogAction::GiveQuest(qid) => alloc::format!("GIVE_QUEST={qid}"),
            DialogAction::CompleteQuest(qid) => alloc::format!("COMPLETE_QUEST={qid}"),
            DialogAction::GiveItem(item_id) => alloc::format!("GIVE_ITEM={item_id}"),
            DialogAction::TakeItem(item_id) => alloc::format!("TAKE_ITEM={item_id}"),
            DialogAction::GiveGold(amount) => alloc::format!("GIVE_GOLD={amount}"),
            DialogAction::TakeGold(amount) => alloc::format!("TAKE_GOLD={amount}"),
            DialogAction::OpenShop(shop_id) => alloc::format!("OPEN_SHOP={shop_id}"),
            DialogAction::Heal => String::from("HEAL"),
        };
        let raw = alloc::format!("@DIALOG:{id}\n{action_token}:line\n@END");

        let parsed = parse_dialogs(&raw);
        let mut dialogs = match parsed {
            Ok(dialogs) => dialogs,
            Err(_) => panic!("failed to build test action dialog"),
        };

        if dialogs.len() == 1 {
            dialogs.remove(0)
        } else {
            panic!("expected exactly one dialog");
        }
    }

    #[test]
    fn back_returns_to_explore() {
        let dialog = make_dialog("dlg_back", 1);
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let dialog_state = DialogState::new(String::from("NPC"), &dialog);

        let event = reduce(Some(&dialog_state), &data, DialogIntent::Back);

        assert!(matches!(
            event,
            DialogEvent::Transition(DialogTransition::CloseToExplore)
        ));
    }

    #[test]
    fn confirm_advances_current_line_when_more_lines_exist() {
        let dialog = make_dialog("dlg_advance", 2);
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let dialog_state = DialogState::new(String::from("NPC"), &dialog);

        let event = reduce(Some(&dialog_state), &data, DialogIntent::Confirm);

        let DialogEvent::Transition(DialogTransition::Set(dialog_state)) = event else {
            panic!("expected dialog state");
        };
        assert_eq!(dialog_state.current_line, 1);
    }

    #[test]
    fn confirm_closes_dialog_on_last_line_without_action() {
        let dialog = make_dialog("dlg_last", 1);
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let dialog_state = DialogState::new(String::from("NPC"), &dialog);

        let event = reduce(Some(&dialog_state), &data, DialogIntent::Confirm);

        assert!(matches!(
            event,
            DialogEvent::Transition(DialogTransition::CloseToExplore)
        ));
    }

    #[test]
    fn confirm_open_shop_action_transitions_to_shop() {
        let dialog =
            make_dialog_with_action("dlg_shop", DialogAction::OpenShop(String::from("s1")));
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());
        data.shops.push(Shop {
            id: String::from("s1"),
            name: String::from("Shop"),
            items: vec![],
        });

        let dialog_state = DialogState::new(String::from("NPC"), &dialog);

        let event = reduce(Some(&dialog_state), &data, DialogIntent::Confirm);

        let DialogEvent::Action(DialogAction::OpenShop(shop_id), DialogTransition::CloseToExplore) =
            event
        else {
            panic!("expected open shop action event");
        };
        assert_eq!(shop_id, "s1");
    }

    #[test]
    fn confirm_give_quest_action_returns_action_event() {
        let dialog =
            make_dialog_with_action("dlg_quest", DialogAction::GiveQuest(String::from("q1")));
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let dialog_state = DialogState::new(String::from("NPC"), &dialog);

        let event = reduce(Some(&dialog_state), &data, DialogIntent::Confirm);

        assert!(matches!(
            event,
            DialogEvent::Action(DialogAction::GiveQuest(_), DialogTransition::CloseToExplore)
        ));
    }

    #[test]
    fn intent_for_key_maps_expected_keys() {
        assert!(matches!(
            DialogIntent::intent_for_key(KeyCode::Ok),
            Some(DialogIntent::Confirm)
        ));
        assert!(matches!(
            DialogIntent::intent_for_key(KeyCode::Back),
            Some(DialogIntent::Back)
        ));
        assert!(matches!(DialogIntent::intent_for_key(KeyCode::Up), None));
    }
}

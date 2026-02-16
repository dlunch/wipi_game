use crate::data::DialogAction;
use crate::game::{DialogState, InputKey};

#[derive(Debug, Clone, Copy)]
pub enum DialogIntent {
    Confirm,
    Back,
}

impl DialogIntent {
    pub fn intent_for_key(key: InputKey) -> Option<DialogIntent> {
        match key {
            InputKey::Ok => Some(DialogIntent::Confirm),
            InputKey::Back => Some(DialogIntent::Back),
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
    SetLine(usize),
    CloseToExplore,
}

pub fn resolve(dialog_state: Option<&DialogState>, intent: DialogIntent) -> DialogEvent {
    match intent {
        DialogIntent::Confirm => {
            if let Some(dialog_state_ref) = dialog_state {
                if dialog_state_ref.current_line >= dialog_state_ref.lines.len() {
                    return DialogEvent::Transition(DialogTransition::CloseToExplore);
                }

                let transition = if dialog_state_ref.current_line + 1 < dialog_state_ref.lines.len()
                {
                    DialogTransition::SetLine(dialog_state_ref.current_line + 1)
                } else {
                    DialogTransition::CloseToExplore
                };

                if let Some(action) = dialog_state_ref
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

    use super::{DialogEvent, DialogIntent, DialogTransition, resolve};
    use crate::data::{Dialog, DialogAction, parse_dialogs};
    use crate::game::{DialogState, InputKey};

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

        let dialog_state = DialogState::from_dialog(String::from("NPC"), &dialog);

        let event = resolve(Some(&dialog_state), DialogIntent::Back);

        assert!(matches!(
            event,
            DialogEvent::Transition(DialogTransition::CloseToExplore)
        ));
    }

    #[test]
    fn confirm_advances_current_line_when_more_lines_exist() {
        let dialog = make_dialog("dlg_advance", 2);

        let dialog_state = DialogState::from_dialog(String::from("NPC"), &dialog);

        let event = resolve(Some(&dialog_state), DialogIntent::Confirm);

        let DialogEvent::Transition(DialogTransition::SetLine(line)) = event else {
            panic!("expected set line event");
        };
        assert_eq!(line, 1);
    }

    #[test]
    fn confirm_closes_dialog_on_last_line_without_action() {
        let dialog = make_dialog("dlg_last", 1);

        let dialog_state = DialogState::from_dialog(String::from("NPC"), &dialog);

        let event = resolve(Some(&dialog_state), DialogIntent::Confirm);

        assert!(matches!(
            event,
            DialogEvent::Transition(DialogTransition::CloseToExplore)
        ));
    }

    #[test]
    fn confirm_open_shop_action_transitions_to_shop() {
        let dialog =
            make_dialog_with_action("dlg_shop", DialogAction::OpenShop(String::from("s1")));

        let dialog_state = DialogState::from_dialog(String::from("NPC"), &dialog);

        let event = resolve(Some(&dialog_state), DialogIntent::Confirm);

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

        let dialog_state = DialogState::from_dialog(String::from("NPC"), &dialog);

        let event = resolve(Some(&dialog_state), DialogIntent::Confirm);

        assert!(matches!(
            event,
            DialogEvent::Action(DialogAction::GiveQuest(_), DialogTransition::CloseToExplore)
        ));
    }

    #[test]
    fn intent_for_key_maps_expected_keys() {
        assert!(matches!(
            DialogIntent::intent_for_key(InputKey::Ok),
            Some(DialogIntent::Confirm)
        ));
        assert!(matches!(
            DialogIntent::intent_for_key(InputKey::Back),
            Some(DialogIntent::Back)
        ));
        assert!(matches!(DialogIntent::intent_for_key(InputKey::Up), None));
    }
}

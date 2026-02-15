use wipi::event::KeyCode;

use super::npc;
use crate::game::{DialogState, GameData, GameState, NpcIntent, PlayerState, ShopState};

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
    OpenShop(ShopState),
}

pub fn reduce(
    state: &mut GameState,
    dialog_state: &mut Option<DialogState>,
    player: &mut PlayerState,
    data: &GameData,
    intent: DialogIntent,
) -> DialogEvent {
    match intent {
        DialogIntent::Confirm => {
            if let GameState::Dialog = *state
                && let Some(dialog_state_ref) = dialog_state.as_ref()
                && let Some(dialog) = data.find_dialog(&dialog_state_ref.dialog_id)
                && let Some(action) = dialog
                    .lines
                    .get(dialog_state_ref.current_line)
                    .and_then(|line| line.action.as_ref())
                    .cloned()
                && let Some(event) = npc::reduce(
                    player,
                    data,
                    NpcIntent::ProcessDialogAction { action: &action },
                )
            {
                match event {
                    npc::NpcEvent::OpenShop(shop_state) => {
                        *state = GameState::Shop;
                        return DialogEvent::OpenShop(shop_state);
                    }
                    npc::NpcEvent::OpenDialog(new_dialog_state) => {
                        *dialog_state = Some(new_dialog_state);
                        *state = GameState::Dialog;
                    }
                }
            }

            if matches!(*state, GameState::Shop) {
                return DialogEvent::None;
            }

            if let GameState::Dialog = *state
                && let Some(dialog_state_ref) = dialog_state.as_mut()
                && let Some(dialog) = data.find_dialog(&dialog_state_ref.dialog_id)
                && !(if dialog_state_ref.current_line + 1 < dialog.lines.len() {
                    dialog_state_ref.current_line += 1;
                    true
                } else {
                    false
                })
            {
                *dialog_state = None;
                *state = GameState::Explore;
            }
        }
        DialogIntent::Back => {
            *dialog_state = None;
            *state = GameState::Explore;
        }
    }

    DialogEvent::None
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;

    use wipi::event::KeyCode;

    use super::{DialogEvent, DialogIntent, reduce};
    use crate::data::{Dialog, DialogAction, Shop, parse_dialogs};
    use crate::game::{DialogState, GameData, GameState, PlayerState, UiState};

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

    fn make_player() -> PlayerState {
        PlayerState::new(String::from("H"), "v")
    }

    #[test]
    fn back_returns_to_explore() {
        let dialog = make_dialog("dlg_back", 1);
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let mut player = make_player();
        let mut ui = UiState::default();
        ui.dialog.state = Some(DialogState::new(String::from("NPC"), &dialog));
        let mut state = GameState::Dialog;

        let _ = reduce(
            &mut state,
            &mut ui.dialog.state,
            &mut player,
            &data,
            DialogIntent::Back,
        );

        assert!(matches!(state, GameState::Explore));
    }

    #[test]
    fn confirm_advances_current_line_when_more_lines_exist() {
        let dialog = make_dialog("dlg_advance", 2);
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let mut player = make_player();
        let mut ui = UiState::default();
        ui.dialog.state = Some(DialogState::new(String::from("NPC"), &dialog));
        let mut state = GameState::Dialog;

        let _ = reduce(
            &mut state,
            &mut ui.dialog.state,
            &mut player,
            &data,
            DialogIntent::Confirm,
        );

        assert!(matches!(state, GameState::Dialog));
        let Some(dialog_state) = ui.dialog.state.as_ref() else {
            panic!("expected dialog state");
        };
        assert_eq!(dialog_state.current_line, 1);
    }

    #[test]
    fn confirm_closes_dialog_on_last_line_without_action() {
        let dialog = make_dialog("dlg_last", 1);
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let mut player = make_player();
        let mut ui = UiState::default();
        ui.dialog.state = Some(DialogState::new(String::from("NPC"), &dialog));
        let mut state = GameState::Dialog;

        let _ = reduce(
            &mut state,
            &mut ui.dialog.state,
            &mut player,
            &data,
            DialogIntent::Confirm,
        );

        assert!(matches!(state, GameState::Explore));
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

        let mut player = make_player();
        let mut ui = UiState::default();
        ui.dialog.state = Some(DialogState::new(String::from("NPC"), &dialog));
        let mut state = GameState::Dialog;

        let event = reduce(
            &mut state,
            &mut ui.dialog.state,
            &mut player,
            &data,
            DialogIntent::Confirm,
        );

        assert!(matches!(state, GameState::Shop));
        let DialogEvent::OpenShop(shop_state) = event else {
            panic!("expected shop state");
        };
        assert_eq!(shop_state.shop.id, "s1");
    }

    #[test]
    fn confirm_give_quest_action_adds_quest_to_player() {
        let dialog =
            make_dialog_with_action("dlg_quest", DialogAction::GiveQuest(String::from("q1")));
        let mut data = GameData::default();
        data.dialogs.push(dialog.clone());

        let mut player = make_player();
        let mut ui = UiState::default();
        ui.dialog.state = Some(DialogState::new(String::from("NPC"), &dialog));
        let mut state = GameState::Dialog;

        let _ = reduce(
            &mut state,
            &mut ui.dialog.state,
            &mut player,
            &data,
            DialogIntent::Confirm,
        );

        assert!(player.has_quest("q1"));
        assert!(matches!(state, GameState::Explore));
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

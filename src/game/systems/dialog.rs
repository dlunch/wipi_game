use crate::data::DialogAction;
use anyhow::{Result, anyhow, ensure};

use crate::game::DialogState;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameState, RuntimeEvent};

#[derive(Debug, Clone, Copy)]
pub enum DialogIntent {
    Confirm,
    Back,
}

#[derive(Debug, Clone)]
pub enum DialogEvent {
    None,
    Transition(DialogTransition),
    Action(DialogAction, DialogTransition),
}

#[derive(Debug, Clone, Copy)]
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

pub fn resolve_many(
    dialog_state: Option<&DialogState>,
    intent: DialogIntent,
) -> alloc::vec::Vec<DialogEvent> {
    match resolve(dialog_state, intent) {
        DialogEvent::None => alloc::vec::Vec::new(),
        event => alloc::vec![event],
    }
}

struct DialogInputResolver;
struct DialogCascadeResolver;

static DIALOG_INPUT_RESOLVER: DialogInputResolver = DialogInputResolver;
static DIALOG_CASCADE_RESOLVER: DialogCascadeResolver = DialogCascadeResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&DIALOG_INPUT_RESOLVER, &DIALOG_CASCADE_RESOLVER]
}

impl DomainEventResolver for DialogInputResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::DialogInput(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::DialogInput(intent) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        ensure!(
            matches!(ctx.state, GameState::Dialog),
            "Invalid state: expected Dialog"
        );
        ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        Ok(resolve_many(ctx.ui.dialog.state.as_ref(), *intent)
            .into_iter()
            .map(RuntimeEvent::Dialog)
            .collect())
    }
}

impl DomainEventResolver for DialogCascadeResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Dialog(_))
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::Dialog(dialog_event) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        match dialog_event {
            DialogEvent::None => Ok(alloc::vec::Vec::new()),
            DialogEvent::Transition(transition) => {
                Ok(alloc::vec![RuntimeEvent::ApplyDialogTransition(
                    *transition
                )])
            }
            DialogEvent::Action(action, transition) => {
                let mut events = alloc::vec![RuntimeEvent::ApplyDialogTransition(*transition)];
                match action {
                    DialogAction::OpenShop(shop_id) => {
                        events.push(RuntimeEvent::OpenShopById(shop_id.clone()));
                    }
                    _ => events.push(RuntimeEvent::ApplyDialogAction(action.clone())),
                }
                Ok(events)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::{DialogEvent, DialogIntent, DialogTransition, resolve};
    use crate::data::{Dialog, DialogAction, parse_dialogs};
    use crate::game::DialogState;

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
}

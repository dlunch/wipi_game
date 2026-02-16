use alloc::vec;
use alloc::vec::Vec;

use crate::data::DialogAction;
use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{DialogCommand, GameEvent, GameState};

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

struct DialogCascadeResolver;
struct DialogInputResolver;

static DIALOG_CASCADE_RESOLVER: DialogCascadeResolver = DialogCascadeResolver;
static DIALOG_INPUT_RESOLVER: DialogInputResolver = DialogInputResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&DIALOG_INPUT_RESOLVER, &DIALOG_CASCADE_RESOLVER]
}

impl DomainEventResolver for DialogInputResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::DialogCommand(_))
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::DialogCommand(input) = event else {
            return Err(anyhow!("Invalid event: expected DialogCommand"));
        };
        ensure!(
            matches!(ctx.state, GameState::Dialog),
            "Invalid state: expected Dialog"
        );

        let event = match input {
            DialogCommand::Back => DialogEvent::Transition(DialogTransition::CloseToExplore),
            DialogCommand::Confirm => {
                if let Some(dialog_state_ref) = ctx.ui.dialog.state.as_ref() {
                    if dialog_state_ref.current_line >= dialog_state_ref.lines.len() {
                        DialogEvent::Transition(DialogTransition::CloseToExplore)
                    } else {
                        let transition =
                            if dialog_state_ref.current_line + 1 < dialog_state_ref.lines.len() {
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
                            DialogEvent::Action(action, transition)
                        } else {
                            DialogEvent::Transition(transition)
                        }
                    }
                } else {
                    DialogEvent::None
                }
            }
        };

        match event {
            DialogEvent::None => Ok(Vec::new()),
            event => Ok(vec![GameEvent::Dialog(event)]),
        }
    }
}

impl DomainEventResolver for DialogCascadeResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Dialog(_))
    }

    fn resolve(&self, _ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::Dialog(dialog_event) = event else {
            return Err(anyhow!("Invalid event: expected Dialog"));
        };
        match dialog_event {
            DialogEvent::None => Ok(Vec::new()),
            DialogEvent::Transition(transition) => {
                Ok(vec![GameEvent::ApplyDialogTransition(*transition)])
            }
            DialogEvent::Action(action, transition) => {
                let mut events = vec![GameEvent::ApplyDialogTransition(*transition)];
                match action {
                    DialogAction::OpenShop(shop_id) => {
                        events.push(GameEvent::OpenShopById(shop_id.clone()));
                    }
                    _ => events.push(GameEvent::ApplyDialogAction(action.clone())),
                }
                Ok(events)
            }
        }
    }
}

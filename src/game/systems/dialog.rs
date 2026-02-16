use alloc::vec::Vec;

use crate::data::DialogAction;
use anyhow::{Result, anyhow, ensure};

use crate::game::GameEvent;
use crate::game::GameState;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};

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
    alloc::vec![&DIALOG_INPUT_RESOLVER, &DIALOG_CASCADE_RESOLVER]
}

impl DomainEventResolver for DialogInputResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::DialogInput(_))
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::DialogInput(key) = event else {
            return Err(anyhow!("Invalid event: expected DialogInput"));
        };
        ensure!(
            matches!(ctx.state, GameState::Dialog),
            "Invalid state: expected Dialog"
        );

        let event = match key {
            crate::game::InputKey::Back => {
                DialogEvent::Transition(DialogTransition::CloseToExplore)
            }
            crate::game::InputKey::Ok => {
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
            _ => DialogEvent::None,
        };

        match event {
            DialogEvent::None => Ok(alloc::vec::Vec::new()),
            event => Ok(alloc::vec![GameEvent::Dialog(event)]),
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
            DialogEvent::None => Ok(alloc::vec::Vec::new()),
            DialogEvent::Transition(transition) => {
                Ok(alloc::vec![GameEvent::ApplyDialogTransition(*transition)])
            }
            DialogEvent::Action(action, transition) => {
                let mut events = alloc::vec![GameEvent::ApplyDialogTransition(*transition)];
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

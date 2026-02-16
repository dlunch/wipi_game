use crate::data::DialogAction;
use anyhow::Result;

use crate::game::GameEvent;
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

static DIALOG_CASCADE_RESOLVER: DialogCascadeResolver = DialogCascadeResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&DIALOG_CASCADE_RESOLVER]
}

impl DomainEventResolver for DialogCascadeResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Dialog(_))
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
    ) -> Result<alloc::vec::Vec<GameEvent>> {
        let GameEvent::Dialog(dialog_event) = event else {
            return Ok(alloc::vec::Vec::new());
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

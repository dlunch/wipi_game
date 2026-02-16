use crate::data::DialogAction;
use anyhow::Result;

use crate::game::RuntimeEvent;
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

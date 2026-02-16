use alloc::vec;
use alloc::vec::Vec;

use crate::data::DialogAction;
use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{DialogCommand, DialogTransition, GameEvent, GameState};

struct DialogInputResolver;

static DIALOG_INPUT_RESOLVER: DialogInputResolver = DialogInputResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&DIALOG_INPUT_RESOLVER]
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

        match input {
            DialogCommand::Back => Ok(vec![GameEvent::ApplyDialogTransition(
                DialogTransition::CloseToExplore,
            )]),
            DialogCommand::Confirm => {
                if let Some(dialog_state_ref) = ctx.ui.dialog.state.as_ref() {
                    if dialog_state_ref.current_line >= dialog_state_ref.lines.len() {
                        return Ok(vec![GameEvent::ApplyDialogTransition(
                            DialogTransition::CloseToExplore,
                        )]);
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
                            let mut events = vec![GameEvent::ApplyDialogTransition(transition)];
                            match action {
                                DialogAction::OpenShop(shop_id) => {
                                    events.push(GameEvent::OpenShopById(shop_id));
                                }
                                _ => events.push(GameEvent::ApplyDialogAction(action)),
                            }
                            return Ok(events);
                        } else {
                            return Ok(vec![GameEvent::ApplyDialogTransition(transition)]);
                        }
                    }
                }
                Ok(Vec::new())
            }
        }
    }
}

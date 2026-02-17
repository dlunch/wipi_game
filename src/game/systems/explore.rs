use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{ExploreCommand, ExploreEvent, GameEvent, GameEventKind, GameState};

struct ExploreResolver;

static EXPLORE_RESOLVER: ExploreResolver = ExploreResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&EXPLORE_RESOLVER]
}

impl DomainEventResolver for ExploreResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::ExploreCommand, GameEventKind::Explore]
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::ExploreCommand(input) => {
                ensure!(
                    matches!(ctx.state, GameState::Explore),
                    "Invalid state: expected Explore"
                );
                let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

                match input {
                    ExploreCommand::Move(direction) => {
                        out.push(GameEvent::Explore(ExploreEvent::MoveDirection(*direction)));
                    }
                    ExploreCommand::Confirm => {
                        let is_peaceful = ctx
                            .data()
                            .find_map(&s.leader.current_map_id)
                            .is_some_and(|map| map.peaceful);
                        out.push(GameEvent::Explore(ExploreEvent::TryNpcInteract {
                            facing: s.leader.facing,
                            fallback_action: if is_peaceful {
                                None
                            } else {
                                Some(ctx.ui.explore.ok_action)
                            },
                        }));
                    }
                    ExploreCommand::UseSlot(slot) => {
                        if let Some(action) = ctx.ui.explore.key_actions.get(*slot).and_then(|a| *a)
                        {
                            out.push(GameEvent::Explore(ExploreEvent::UseAction(action)));
                        }
                    }
                    ExploreCommand::OpenPauseMenu => {
                        out.push(GameEvent::Explore(ExploreEvent::EnterPauseMenu));
                    }
                    ExploreCommand::OpenMenu => {
                        out.push(GameEvent::Explore(ExploreEvent::EnterMenu));
                    }
                }
            }
            GameEvent::Explore(ExploreEvent::UseAction(action)) => {
                out.push(GameEvent::CombatPlayerAction(*action));
            }
            GameEvent::Explore(ExploreEvent::EnterPauseMenu) => {
                out.push(GameEvent::Transition(
                    crate::game::TransitionEvent::ToPauseMenu,
                ));
            }
            GameEvent::Explore(ExploreEvent::EnterMenu) => {
                out.push(GameEvent::SaveSession);
                out.push(GameEvent::Transition(crate::game::TransitionEvent::ToMenu));
            }
            _ => {}
        }
        Ok(())
    }
}

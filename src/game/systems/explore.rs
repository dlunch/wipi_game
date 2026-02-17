use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{ExploreCommand, ExploreEvent, GameEvent, GameEventKind, GameState};

struct ExploreInputResolver;
struct ExploreUseActionCascadeResolver;
struct ExplorePauseCascadeResolver;
struct ExploreMenuCascadeResolver;

static EXPLORE_INPUT_RESOLVER: ExploreInputResolver = ExploreInputResolver;
static EXPLORE_USE_ACTION_CASCADE_RESOLVER: ExploreUseActionCascadeResolver =
    ExploreUseActionCascadeResolver;
static EXPLORE_PAUSE_CASCADE_RESOLVER: ExplorePauseCascadeResolver = ExplorePauseCascadeResolver;
static EXPLORE_MENU_CASCADE_RESOLVER: ExploreMenuCascadeResolver = ExploreMenuCascadeResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![
        &EXPLORE_INPUT_RESOLVER,
        &EXPLORE_USE_ACTION_CASCADE_RESOLVER,
        &EXPLORE_PAUSE_CASCADE_RESOLVER,
        &EXPLORE_MENU_CASCADE_RESOLVER,
    ]
}

impl DomainEventResolver for ExploreInputResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::ExploreCommand]
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let GameEvent::ExploreCommand(input) = event else {
            return Err(anyhow!("Invalid event: expected ExploreCommand"));
        };
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
                if let Some(action) = ctx.ui.explore.key_actions.get(*slot).and_then(|a| *a) {
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
        Ok(())
    }
}

impl DomainEventResolver for ExploreUseActionCascadeResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::Explore]
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let GameEvent::Explore(ExploreEvent::UseAction(action)) = event else {
            return Ok(());
        };
        out.push(GameEvent::CombatPlayerAction(*action));
        Ok(())
    }
}

impl DomainEventResolver for ExplorePauseCascadeResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::Explore]
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        _event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        if !matches!(_event, GameEvent::Explore(ExploreEvent::EnterPauseMenu)) {
            return Ok(());
        }
        out.push(GameEvent::Transition(
            crate::game::TransitionEvent::ToPauseMenu,
        ));
        Ok(())
    }
}

impl DomainEventResolver for ExploreMenuCascadeResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::Explore]
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        _event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        if !matches!(_event, GameEvent::Explore(ExploreEvent::EnterMenu)) {
            return Ok(());
        }
        out.push(GameEvent::SaveSession);
        out.push(GameEvent::Transition(crate::game::TransitionEvent::ToMenu));
        Ok(())
    }
}

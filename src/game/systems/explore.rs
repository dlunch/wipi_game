use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{AppExploreEvent, GameEvent, GameState};

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
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::ExploreInput(_))
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::ExploreInput(key) = event else {
            return Err(anyhow!("Invalid event: expected ExploreInput"));
        };
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        Ok(ctx
            .ui
            .explore
            .resolve_events_for_key(*key, &s.leader, ctx.data())
            .into_iter()
            .map(GameEvent::Explore)
            .collect())
    }
}

impl DomainEventResolver for ExploreUseActionCascadeResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Explore(AppExploreEvent::UseAction(_)))
    }

    fn resolve(&self, _ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::Explore(AppExploreEvent::UseAction(action)) = event else {
            return Err(anyhow!("Invalid event: expected Explore(UseAction)"));
        };
        Ok(vec![GameEvent::CombatPlayerAction(*action)])
    }
}

impl DomainEventResolver for ExplorePauseCascadeResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Explore(AppExploreEvent::EnterPauseMenu))
    }

    fn resolve(&self, _ctx: &mut ResolveContext<'_>, _event: &GameEvent) -> Result<Vec<GameEvent>> {
        Ok(vec![GameEvent::OpenPauseMenu])
    }
}

impl DomainEventResolver for ExploreMenuCascadeResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Explore(AppExploreEvent::EnterMenu))
    }

    fn resolve(&self, _ctx: &mut ResolveContext<'_>, _event: &GameEvent) -> Result<Vec<GameEvent>> {
        Ok(vec![GameEvent::OpenMenuFromExplore])
    }
}

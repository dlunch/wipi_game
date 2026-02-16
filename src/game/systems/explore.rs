use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{AppExploreEvent, ExploreInputEvent, GameEvent, GameState};

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
        let GameEvent::ExploreInput(input) = event else {
            return Err(anyhow!("Invalid event: expected ExploreInput"));
        };
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        let mut out = Vec::new();
        match input {
            ExploreInputEvent::Move(direction) => {
                out.push(GameEvent::Explore(AppExploreEvent::MoveDirection(
                    *direction,
                )));
            }
            ExploreInputEvent::Confirm => {
                let is_peaceful = ctx
                    .data()
                    .find_map(&s.leader.current_map_id)
                    .is_some_and(|map| map.peaceful);
                out.push(GameEvent::Explore(AppExploreEvent::TryNpcInteract {
                    facing: s.leader.facing,
                    fallback_action: if is_peaceful {
                        None
                    } else {
                        Some(ctx.ui.explore.ok_action)
                    },
                }));
            }
            ExploreInputEvent::UseSlot(slot) => {
                if let Some(action) = ctx.ui.explore.key_actions.get(*slot).and_then(|a| *a) {
                    out.push(GameEvent::Explore(AppExploreEvent::UseAction(action)));
                }
            }
            ExploreInputEvent::OpenPauseMenu => {
                out.push(GameEvent::Explore(AppExploreEvent::EnterPauseMenu));
            }
            ExploreInputEvent::OpenMenu => {
                out.push(GameEvent::Explore(AppExploreEvent::EnterMenu));
            }
        }
        Ok(out)
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

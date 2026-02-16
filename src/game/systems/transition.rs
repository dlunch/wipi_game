use anyhow::Result;

use crate::game::TransitionEvent;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};

struct OverlayCloseResolver;
struct GameOverConfirmResolver;
struct ErrorConfirmResolver;

static OVERLAY_CLOSE_RESOLVER: OverlayCloseResolver = OverlayCloseResolver;
static GAME_OVER_CONFIRM_RESOLVER: GameOverConfirmResolver = GameOverConfirmResolver;
static ERROR_CONFIRM_RESOLVER: ErrorConfirmResolver = ErrorConfirmResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![
        &OVERLAY_CLOSE_RESOLVER,
        &GAME_OVER_CONFIRM_RESOLVER,
        &ERROR_CONFIRM_RESOLVER,
    ]
}

impl DomainEventResolver for OverlayCloseResolver {
    fn handles(&self, event: &crate::game::RuntimeEvent) -> bool {
        matches!(event, crate::game::RuntimeEvent::OverlayCloseRequested)
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        _event: &crate::game::RuntimeEvent,
    ) -> Result<alloc::vec::Vec<crate::game::RuntimeEvent>> {
        Ok(alloc::vec![crate::game::RuntimeEvent::Transition(
            TransitionEvent::ToExplore,
        )])
    }
}

impl DomainEventResolver for GameOverConfirmResolver {
    fn handles(&self, event: &crate::game::RuntimeEvent) -> bool {
        matches!(event, crate::game::RuntimeEvent::GameOverConfirmRequested)
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        _event: &crate::game::RuntimeEvent,
    ) -> Result<alloc::vec::Vec<crate::game::RuntimeEvent>> {
        Ok(alloc::vec![crate::game::RuntimeEvent::Transition(
            TransitionEvent::ToMenuFromGameOver,
        )])
    }
}

impl DomainEventResolver for ErrorConfirmResolver {
    fn handles(&self, event: &crate::game::RuntimeEvent) -> bool {
        matches!(event, crate::game::RuntimeEvent::ErrorConfirmRequested)
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        _event: &crate::game::RuntimeEvent,
    ) -> Result<alloc::vec::Vec<crate::game::RuntimeEvent>> {
        Ok(alloc::vec![crate::game::RuntimeEvent::Exit(1)])
    }
}

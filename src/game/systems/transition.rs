use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{
    ApplyContext, DomainEventApplier, DomainEventResolver, ResolveContext,
};
use crate::game::{GameState, MenuState, TransitionEvent, has_save_data};

struct OverlayCloseResolver;
struct GameOverConfirmResolver;
struct ErrorConfirmResolver;
struct TransitionApplier;
struct ExitApplier;

static OVERLAY_CLOSE_RESOLVER: OverlayCloseResolver = OverlayCloseResolver;
static GAME_OVER_CONFIRM_RESOLVER: GameOverConfirmResolver = GameOverConfirmResolver;
static ERROR_CONFIRM_RESOLVER: ErrorConfirmResolver = ErrorConfirmResolver;
static TRANSITION_APPLIER: TransitionApplier = TransitionApplier;
static EXIT_APPLIER: ExitApplier = ExitApplier;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![
        &OVERLAY_CLOSE_RESOLVER,
        &GAME_OVER_CONFIRM_RESOLVER,
        &ERROR_CONFIRM_RESOLVER,
    ]
}

pub fn appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&TRANSITION_APPLIER, &EXIT_APPLIER]
}

fn apply_transition(ctx: &mut ApplyContext<'_>, event: TransitionEvent) -> Result<()> {
    match event {
        TransitionEvent::MapChanged => apply_map_changed(ctx)?,
        TransitionEvent::ToExplore => ctx.transition_to(GameState::Explore),
        TransitionEvent::ToMenuFromGameOver => {
            ctx.transition_to(GameState::Menu);
            ctx.ui_mut().menu.set_menu(MenuState::new(has_save_data()));
        }
        TransitionEvent::ReleaseMovementDirection(direction) => {
            apply_release_movement_direction(ctx, direction)?
        }
    }

    Ok(())
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

impl DomainEventApplier for TransitionApplier {
    fn handles(&self, event: &crate::game::RuntimeEvent) -> bool {
        matches!(event, crate::game::RuntimeEvent::Transition(_))
    }

    fn apply(
        &self,
        engine: &mut ApplyContext<'_>,
        event: &crate::game::RuntimeEvent,
    ) -> Result<()> {
        let crate::game::RuntimeEvent::Transition(transition) = event else {
            return Ok(());
        };
        apply_transition(engine, *transition)
    }
}

impl DomainEventApplier for ExitApplier {
    fn handles(&self, event: &crate::game::RuntimeEvent) -> bool {
        matches!(event, crate::game::RuntimeEvent::Exit(_))
    }

    fn apply(
        &self,
        _engine: &mut ApplyContext<'_>,
        event: &crate::game::RuntimeEvent,
    ) -> Result<()> {
        let crate::game::RuntimeEvent::Exit(code) = event else {
            return Ok(());
        };
        wipi::kernel::exit(*code);
        Ok(())
    }
}

fn apply_map_changed(ctx: &mut ApplyContext<'_>) -> Result<()> {
    let data = ctx.data_rc();
    let s = ctx
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.spawn_current_map_enemies(&data);
    Ok(())
}

fn apply_release_movement_direction(
    ctx: &mut ApplyContext<'_>,
    direction: crate::data::Direction,
) -> Result<()> {
    ensure!(
        matches!(ctx.state, GameState::Explore),
        "Invalid state: expected Explore"
    );

    let s = ctx
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.on_direction_released(direction);
    Ok(())
}

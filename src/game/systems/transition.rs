use anyhow::{Result, anyhow, ensure};

use crate::engine::GameEngine;
use crate::game::systems::runtime::{DomainEventApplier, DomainEventResolver};
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

fn apply_transition(engine: &mut GameEngine, event: TransitionEvent) -> Result<()> {
    match event {
        TransitionEvent::MapChanged => apply_map_changed(engine)?,
        TransitionEvent::ToExplore => engine.transition_to(GameState::Explore),
        TransitionEvent::ToMenuFromGameOver => {
            engine.transition_to(GameState::Menu);
            engine
                .ui_mut()
                .menu
                .set_menu(MenuState::new(has_save_data()));
        }
        TransitionEvent::ReleaseMovementDirection(direction) => {
            apply_release_movement_direction(engine, direction)?
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
        _engine: &mut GameEngine,
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
        _engine: &mut GameEngine,
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
        _engine: &mut GameEngine,
        _event: &crate::game::RuntimeEvent,
    ) -> Result<alloc::vec::Vec<crate::game::RuntimeEvent>> {
        Ok(alloc::vec![crate::game::RuntimeEvent::Exit(1)])
    }
}

impl DomainEventApplier for TransitionApplier {
    fn handles(&self, event: &crate::game::RuntimeEvent) -> bool {
        matches!(event, crate::game::RuntimeEvent::Transition(_))
    }

    fn apply(&self, engine: &mut GameEngine, event: &crate::game::RuntimeEvent) -> Result<()> {
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

    fn apply(&self, _engine: &mut GameEngine, event: &crate::game::RuntimeEvent) -> Result<()> {
        let crate::game::RuntimeEvent::Exit(code) = event else {
            return Ok(());
        };
        wipi::kernel::exit(*code);
        Ok(())
    }
}

fn apply_map_changed(engine: &mut GameEngine) -> Result<()> {
    let data = engine.data_rc();
    let s = engine
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.spawn_current_map_enemies(&data);
    Ok(())
}

fn apply_release_movement_direction(
    engine: &mut GameEngine,
    direction: crate::data::Direction,
) -> Result<()> {
    ensure!(
        matches!(engine.state(), GameState::Explore),
        "Invalid state: expected Explore"
    );

    let s = engine
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.on_direction_released(direction);
    Ok(())
}

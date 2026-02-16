use anyhow::{Result, anyhow, ensure};

use crate::engine::GameEngine;
use crate::game::{GameState, MenuState, TransitionEvent, has_save_data};

pub fn apply(engine: &mut GameEngine, event: TransitionEvent) -> Result<()> {
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

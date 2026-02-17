use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use super::save::save_game;
use crate::game::systems::lifecycle::emit_continue_setup_events;
use crate::game::{
    GameData, GameEvent, GameState, LifecycleEvent, LoadingEvent, TransitionEvent, WorldState,
};

pub fn apply_effects(
    state: &GameState,
    data: &mut Rc<GameData>,
    world: Option<&WorldState>,
    event: &GameEvent,
) -> Result<Vec<GameEvent>> {
    let mut out = Vec::with_capacity(4);
    match event {
        GameEvent::Loading(LoadingEvent::Tick) => {
            let step = match state {
                GameState::Loading(step) => *step,
                _ => {
                    out.push(GameEvent::Loading(LoadingEvent::Error(String::from(
                        "Invalid state: expected Loading",
                    ))));
                    return Ok(out);
                }
            };

            let load_result = if let Some(data_mut) = Rc::get_mut(data) {
                data_mut
                    .load_step(step)
                    .map_err(|e| format!("Load error: {}", e))
            } else {
                Err(String::from("Load error: data is shared"))
            };

            let loading = match load_result {
                Ok(true) => LoadingEvent::Loaded,
                Ok(false) => LoadingEvent::Advance(step + 1),
                Err(e) => LoadingEvent::Error(e),
            };
            out.push(GameEvent::Loading(loading));
        }
        GameEvent::SaveWorld => {
            let world = world.ok_or_else(|| anyhow!("No active world"))?;
            save_game(world)?;
        }
        GameEvent::Lifecycle(LifecycleEvent::ContinueSetup) => {
            emit_continue_setup_events(data.as_ref(), &mut out);
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
        GameEvent::Exit(code) => {
            wipi::kernel::exit(*code);
        }
        _ => {}
    }
    Ok(out)
}

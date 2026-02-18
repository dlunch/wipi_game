use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use super::save::{has_save_data, save_game};
use crate::game::{
    GameData, GameEvent, GameState, LifecycleEvent, LoadingEvent, TransitionEvent, WorldState,
};

pub fn apply_effects(
    state: &GameState,
    data: &mut Rc<GameData>,
    world: Option<&WorldState>,
    event: &GameEvent,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    match event {
        GameEvent::Loading(LoadingEvent::Tick) => {
            let step = match state {
                GameState::Loading(step) => *step,
                _ => {
                    out.push(GameEvent::Loading(LoadingEvent::Error(String::from(
                        "Invalid state: expected Loading",
                    ))));
                    return Ok(());
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
                Ok(true) => {
                    out.push(GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(
                        has_save_data(),
                    )));
                    LoadingEvent::Loaded
                }
                Ok(false) => LoadingEvent::Advance(step + 1),
                Err(e) => LoadingEvent::Error(e),
            };
            out.push(GameEvent::Loading(loading));
        }
        GameEvent::SaveWorld => {
            let world = world.ok_or_else(|| anyhow!("No active world"))?;
            save_game(world)?;
            out.push(GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(
                true,
            )));
        }
        GameEvent::Transition(TransitionEvent::ToMenu) => {
            out.push(GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(
                has_save_data(),
            )));
        }
        GameEvent::Exit(code) => {
            wipi::kernel::exit(*code);
        }
        _ => {}
    }
    Ok(())
}

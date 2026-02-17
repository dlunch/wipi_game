use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::{GameData, GameEvent, GameState, LoadingEvent, WorldState, save_game};

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
        _ => {}
    }
    Ok(())
}

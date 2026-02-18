use alloc::rc::Rc;
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
        GameEvent::Tick => {
            let GameState::Loading(step) = state else {
                return Ok(());
            };

            let data_mut =
                Rc::get_mut(data).ok_or_else(|| anyhow!("Load error: data is shared"))?;
            let loaded = data_mut
                .load_step(*step)
                .map_err(|e| anyhow!("Load error: {}", e))?;
            let loading = if loaded {
                out.push(GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(
                    has_save_data()?,
                )));
                LoadingEvent::Loaded
            } else {
                LoadingEvent::Advance(*step + 1)
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
                has_save_data()?,
            )));
        }
        GameEvent::Exit(code) => {
            wipi::kernel::exit(*code);
        }
        _ => {}
    }
    Ok(())
}

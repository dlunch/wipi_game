use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;

use crate::game::{GameData, GameEvent, GameState, LoadingEvent};

pub fn apply_loading_update(
    state: &GameState,
    data: &mut Rc<GameData>,
    event: &GameEvent,
) -> Option<GameEvent> {
    let GameEvent::Loading(LoadingEvent::Tick) = event else {
        return None;
    };

    let step = match state {
        GameState::Loading(step) => *step,
        _ => {
            return Some(GameEvent::Loading(LoadingEvent::Error(String::from(
                "Invalid state: expected Loading",
            ))));
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
    Some(GameEvent::Loading(loading))
}

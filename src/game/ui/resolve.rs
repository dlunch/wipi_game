use alloc::vec;
use alloc::vec::Vec;

use super::state::{GameInput, InputKey, UiEvent, UiState};
use crate::game::{GameState, WorldState};

pub trait UiInputEventResolver {
    fn resolve_input(
        &mut self,
        input: GameInput,
        game_state: &GameState,
        session: Option<&WorldState>,
    ) -> Vec<UiEvent>;
}

impl UiInputEventResolver for UiState {
    fn resolve_input(
        &mut self,
        input: GameInput,
        game_state: &GameState,
        session: Option<&WorldState>,
    ) -> Vec<UiEvent> {
        match input {
            GameInput::KeyDown(key) => resolve_keydown(key, game_state, self, session),
            GameInput::KeyUp(key) => resolve_keyup(key, game_state, session),
        }
    }
}

fn resolve_keydown(
    key: InputKey,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&WorldState>,
) -> Vec<UiEvent> {
    match game_state {
        GameState::Loading(_) => Vec::new(),
        GameState::Menu => ui.menu.event_for_key(key).into_iter().collect(),
        GameState::Explore => ui.explore.events_for_key(key),
        GameState::Inventory => ui.inventory.event_for_key(key).into_iter().collect(),
        GameState::Stats | GameState::QuestLog => {
            if matches!(key, InputKey::Back | InputKey::Ok) {
                vec![UiEvent::OverlayCloseRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Dialog => ui.dialog.event_for_key(key).into_iter().collect(),
        GameState::Shop => {
            let inventory_len = session
                .map(|session_state| session_state.leader.inventory.len())
                .unwrap_or(0);
            ui.shop.handle_key(key, inventory_len).into_iter().collect()
        }
        GameState::PauseMenu => ui.pause_menu.event_for_key(key).into_iter().collect(),
        GameState::GameOver => {
            if matches!(key, InputKey::Ok) {
                vec![UiEvent::GameOverConfirmRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Error(_) => {
            if matches!(key, InputKey::Ok) {
                vec![UiEvent::ErrorConfirmRequested]
            } else {
                Vec::new()
            }
        }
    }
}

fn resolve_keyup(
    key: InputKey,
    game_state: &GameState,
    session: Option<&WorldState>,
) -> Vec<UiEvent> {
    if matches!(game_state, GameState::Explore)
        && session.is_some()
        && let Some(direction) = key.direction()
    {
        vec![UiEvent::MovementKeyReleased(direction)]
    } else {
        Vec::new()
    }
}

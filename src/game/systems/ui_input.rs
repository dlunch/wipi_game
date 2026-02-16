use alloc::vec;
use alloc::vec::Vec;

use crate::data::Direction;
use crate::game::{GameInput, GameState, RuntimeEvent, SessionState, TransitionEvent, UiState};

pub trait UiInputEventResolver {
    fn resolve_input_event(
        &mut self,
        event: &RuntimeEvent,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<RuntimeEvent>;
}

impl UiInputEventResolver for UiState {
    fn resolve_input_event(
        &mut self,
        event: &RuntimeEvent,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<RuntimeEvent> {
        match event {
            RuntimeEvent::Tick => resolve(GameInput::Tick, game_state, self, session),
            RuntimeEvent::KeyDown(key) => {
                resolve(GameInput::KeyDown(*key), game_state, self, session)
            }
            RuntimeEvent::KeyUp(key) => resolve(GameInput::KeyUp(*key), game_state, self, session),
            _ => Vec::new(),
        }
    }
}

pub fn resolve(
    input: GameInput,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<RuntimeEvent> {
    match input {
        GameInput::Tick => resolve_tick(game_state),
        GameInput::KeyDown(key) => resolve_keydown(key, game_state, ui, session),
        GameInput::KeyUp(key) => resolve_keyup(key, game_state, session),
    }
}

fn resolve_tick(game_state: &GameState) -> Vec<RuntimeEvent> {
    match game_state {
        GameState::Loading(_) => vec![RuntimeEvent::UpdateLoading],
        GameState::Explore => vec![RuntimeEvent::UpdateMovement, RuntimeEvent::UpdateCombat],
        _ => Vec::new(),
    }
}

fn resolve_keydown(
    key: crate::game::InputKey,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<RuntimeEvent> {
    match game_state {
        GameState::Loading(_) => Vec::new(),
        GameState::Menu => ui.menu.event_for_key(key).into_iter().collect(),
        GameState::Explore => {
            let facing = session
                .map(|session_state| session_state.player.facing)
                .unwrap_or(Direction::Down);
            ui.explore.events_for_key(key, facing)
        }
        GameState::Inventory => ui.inventory.event_for_key(key).into_iter().collect(),
        GameState::Stats | GameState::QuestLog => {
            if matches!(key, crate::game::InputKey::Back | crate::game::InputKey::Ok) {
                vec![RuntimeEvent::OverlayCloseRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Dialog => ui.dialog.event_for_key(key).into_iter().collect(),
        GameState::Shop => {
            let inventory_len = session
                .map(|session_state| session_state.player.inventory.len())
                .unwrap_or(0);
            ui.shop
                .event_for_key(key, inventory_len)
                .into_iter()
                .collect()
        }
        GameState::PauseMenu => ui.pause_menu.event_for_key(key).into_iter().collect(),
        GameState::GameOver => {
            if matches!(key, crate::game::InputKey::Ok) {
                vec![RuntimeEvent::GameOverConfirmRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Error(_) => {
            if matches!(key, crate::game::InputKey::Ok) {
                vec![RuntimeEvent::ErrorConfirmRequested]
            } else {
                Vec::new()
            }
        }
    }
}

fn resolve_keyup(
    key: crate::game::InputKey,
    game_state: &GameState,
    session: Option<&SessionState>,
) -> Vec<RuntimeEvent> {
    if matches!(game_state, GameState::Explore)
        && session.is_some()
        && let Some(direction) = key.direction()
    {
        vec![RuntimeEvent::Transition(
            TransitionEvent::ReleaseMovementDirection(direction),
        )]
    } else {
        Vec::new()
    }
}

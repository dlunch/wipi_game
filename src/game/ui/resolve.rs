use alloc::vec;
use alloc::vec::Vec;

use super::state::{
    DialogUiState, ExploreUiState, GameInput, InputKey, InventoryUiState, MenuUiState,
    PauseMenuUiState, QuestLogUiState, UiEvent, UiState,
};
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

impl ExploreUiState {
    pub fn events_for_key(&self, key: InputKey) -> Vec<UiEvent> {
        if matches!(
            key,
            InputKey::Up
                | InputKey::Down
                | InputKey::Left
                | InputKey::Right
                | InputKey::Ok
                | InputKey::Key0
                | InputKey::Key1
                | InputKey::Key2
                | InputKey::Key3
                | InputKey::Back
        ) {
            vec![UiEvent::ExploreInput(key)]
        } else {
            Vec::new()
        }
    }
}

impl MenuUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(key, InputKey::Up | InputKey::Down | InputKey::Ok) {
            Some(UiEvent::MenuInput(key))
        } else {
            None
        }
    }
}

impl PauseMenuUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(
            key,
            InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back | InputKey::Key0
        ) {
            Some(UiEvent::PauseMenuInput(key))
        } else {
            None
        }
    }
}

impl InventoryUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(
            key,
            InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back
        ) {
            Some(UiEvent::InventoryInput(key))
        } else {
            None
        }
    }
}

impl QuestLogUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(
            key,
            InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back
        ) {
            Some(UiEvent::QuestLogInput(key))
        } else {
            None
        }
    }
}

impl DialogUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(key, InputKey::Ok | InputKey::Back) {
            Some(UiEvent::DialogInput(key))
        } else {
            None
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
        GameState::Dead => {
            if matches!(key, InputKey::Ok) {
                vec![UiEvent::ReviveRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Inventory => ui.inventory.event_for_key(key).into_iter().collect(),
        GameState::Stats => {
            if matches!(key, InputKey::Back | InputKey::Ok) {
                vec![UiEvent::OverlayCloseRequested]
            } else {
                Vec::new()
            }
        }
        GameState::QuestLog => ui.quest_log.event_for_key(key).into_iter().collect(),
        GameState::Dialog => ui.dialog.event_for_key(key).into_iter().collect(),
        GameState::Shop => {
            let _ = session;
            if matches!(
                key,
                InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back
            ) {
                vec![UiEvent::ShopInput(key)]
            } else {
                Vec::new()
            }
        }
        GameState::PauseMenu => ui.pause_menu.event_for_key(key).into_iter().collect(),
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

#[cfg(test)]
mod tests {
    use super::{
        DialogUiState, ExploreUiState, InputKey, InventoryUiState, MenuUiState, PauseMenuUiState,
        UiEvent,
    };

    #[test]
    fn explore_ui_maps_ok_to_npc_interact_with_fallback_action() {
        let ui = ExploreUiState::default();
        let events = ui.events_for_key(InputKey::Ok);
        assert!(matches!(
            events.as_slice(),
            [UiEvent::ExploreInput(InputKey::Ok)]
        ));
    }

    #[test]
    fn menu_ui_maps_up_down_ok_keys() {
        let ui = MenuUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Up),
            Some(UiEvent::MenuInput(InputKey::Up))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Down),
            Some(UiEvent::MenuInput(InputKey::Down))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::MenuInput(InputKey::Ok))
        ));
    }

    #[test]
    fn pause_menu_ui_maps_back_and_zero_to_back_intent() {
        let ui = PauseMenuUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::PauseMenuInput(InputKey::Back))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Key0),
            Some(UiEvent::PauseMenuInput(InputKey::Key0))
        ));
    }

    #[test]
    fn inventory_ui_maps_expected_keys() {
        let ui = InventoryUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Up),
            Some(UiEvent::InventoryInput(InputKey::Up))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Down),
            Some(UiEvent::InventoryInput(InputKey::Down))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::InventoryInput(InputKey::Ok))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::InventoryInput(InputKey::Back))
        ));
    }

    #[test]
    fn dialog_ui_maps_expected_keys() {
        let ui = DialogUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::DialogInput(InputKey::Ok))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::DialogInput(InputKey::Back))
        ));
    }
}

use alloc::vec;
use alloc::vec::Vec;

use super::state::{GameInput, InputKey, UiEvent, UiState};
use crate::game::state::{GameState, world::WorldState};

const EXPLORE_KEYS: [InputKey; 10] = [
    InputKey::Up,
    InputKey::Down,
    InputKey::Left,
    InputKey::Right,
    InputKey::Ok,
    InputKey::Key0,
    InputKey::Key1,
    InputKey::Key2,
    InputKey::Key3,
    InputKey::Back,
];
const MENU_KEYS: [InputKey; 3] = [InputKey::Up, InputKey::Down, InputKey::Ok];
const PAUSE_MENU_KEYS: [InputKey; 5] = [
    InputKey::Up,
    InputKey::Down,
    InputKey::Ok,
    InputKey::Back,
    InputKey::Key0,
];
const INVENTORY_KEYS: [InputKey; 4] = [InputKey::Up, InputKey::Down, InputKey::Ok, InputKey::Back];
const QUEST_LOG_KEYS: [InputKey; 4] = [InputKey::Up, InputKey::Down, InputKey::Ok, InputKey::Back];
const DIALOG_KEYS: [InputKey; 2] = [InputKey::Ok, InputKey::Back];
const SHOP_KEYS: [InputKey; 4] = [InputKey::Up, InputKey::Down, InputKey::Ok, InputKey::Back];
const OVERLAY_CLOSE_KEYS: [InputKey; 2] = [InputKey::Back, InputKey::Ok];
const OK_KEY: [InputKey; 1] = [InputKey::Ok];

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
            GameInput::KeyDown(key) => resolve_keydown(key, game_state),
            GameInput::KeyUp(key) => resolve_keyup(key, game_state, session),
        }
    }
}

fn key_event_if_allowed(
    key: InputKey,
    allowed: &[InputKey],
    map: fn(InputKey) -> UiEvent,
) -> Option<UiEvent> {
    allowed.contains(&key).then(|| map(key))
}

fn fixed_event_if_allowed(key: InputKey, allowed: &[InputKey], event: UiEvent) -> Option<UiEvent> {
    allowed.contains(&key).then_some(event)
}

fn resolve_keydown(key: InputKey, game_state: &GameState) -> Vec<UiEvent> {
    let event = match game_state {
        GameState::Loading(_) => None,
        GameState::Menu => key_event_if_allowed(key, &MENU_KEYS, UiEvent::MenuInput),
        GameState::Explore => key_event_if_allowed(key, &EXPLORE_KEYS, UiEvent::ExploreInput),
        GameState::Dead => fixed_event_if_allowed(key, &OK_KEY, UiEvent::ReviveRequested),
        GameState::Inventory => key_event_if_allowed(key, &INVENTORY_KEYS, UiEvent::InventoryInput),
        GameState::Stats => {
            fixed_event_if_allowed(key, &OVERLAY_CLOSE_KEYS, UiEvent::OverlayCloseRequested)
        }
        GameState::QuestLog => key_event_if_allowed(key, &QUEST_LOG_KEYS, UiEvent::QuestLogInput),
        GameState::Dialog => key_event_if_allowed(key, &DIALOG_KEYS, UiEvent::DialogInput),
        GameState::Shop => key_event_if_allowed(key, &SHOP_KEYS, UiEvent::ShopInput),
        GameState::PauseMenu => {
            key_event_if_allowed(key, &PAUSE_MENU_KEYS, UiEvent::PauseMenuInput)
        }
        GameState::Error(_) => fixed_event_if_allowed(key, &OK_KEY, UiEvent::ErrorConfirmRequested),
    };
    event.into_iter().collect()
}

fn resolve_keyup(
    key: InputKey,
    game_state: &GameState,
    _session: Option<&WorldState>,
) -> Vec<UiEvent> {
    if matches!(game_state, GameState::Explore)
        && let Some(direction) = key.direction()
    {
        vec![UiEvent::MovementKeyReleased(direction)]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{GameState, InputKey, UiEvent, resolve_keydown};

    #[test]
    fn explore_ui_maps_ok_to_npc_interact_with_fallback_action() {
        let events = resolve_keydown(InputKey::Ok, &GameState::Explore);
        assert!(matches!(
            events.as_slice(),
            [UiEvent::ExploreInput(InputKey::Ok)]
        ));
    }

    #[test]
    fn menu_ui_maps_up_down_ok_keys() {
        assert!(matches!(
            resolve_keydown(InputKey::Up, &GameState::Menu).as_slice(),
            [UiEvent::MenuInput(InputKey::Up)]
        ));
        assert!(matches!(
            resolve_keydown(InputKey::Down, &GameState::Menu).as_slice(),
            [UiEvent::MenuInput(InputKey::Down)]
        ));
        assert!(matches!(
            resolve_keydown(InputKey::Ok, &GameState::Menu).as_slice(),
            [UiEvent::MenuInput(InputKey::Ok)]
        ));
    }

    #[test]
    fn pause_menu_ui_maps_back_and_zero_to_back_intent() {
        assert!(matches!(
            resolve_keydown(InputKey::Back, &GameState::PauseMenu).as_slice(),
            [UiEvent::PauseMenuInput(InputKey::Back)]
        ));
        assert!(matches!(
            resolve_keydown(InputKey::Key0, &GameState::PauseMenu).as_slice(),
            [UiEvent::PauseMenuInput(InputKey::Key0)]
        ));
    }

    #[test]
    fn inventory_ui_maps_expected_keys() {
        assert!(matches!(
            resolve_keydown(InputKey::Up, &GameState::Inventory).as_slice(),
            [UiEvent::InventoryInput(InputKey::Up)]
        ));
        assert!(matches!(
            resolve_keydown(InputKey::Down, &GameState::Inventory).as_slice(),
            [UiEvent::InventoryInput(InputKey::Down)]
        ));
        assert!(matches!(
            resolve_keydown(InputKey::Ok, &GameState::Inventory).as_slice(),
            [UiEvent::InventoryInput(InputKey::Ok)]
        ));
        assert!(matches!(
            resolve_keydown(InputKey::Back, &GameState::Inventory).as_slice(),
            [UiEvent::InventoryInput(InputKey::Back)]
        ));
    }

    #[test]
    fn dialog_ui_maps_expected_keys() {
        assert!(matches!(
            resolve_keydown(InputKey::Ok, &GameState::Dialog).as_slice(),
            [UiEvent::DialogInput(InputKey::Ok)]
        ));
        assert!(matches!(
            resolve_keydown(InputKey::Back, &GameState::Dialog).as_slice(),
            [UiEvent::DialogInput(InputKey::Back)]
        ));
    }
}

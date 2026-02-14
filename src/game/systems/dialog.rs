use wipi::event::KeyCode;

use super::npc;
use crate::game::{GameData, GameState, NpcIntent, PlayerState};

#[derive(Debug, Clone, Copy)]
pub enum DialogIntent {
    Confirm,
    Back,
}

impl DialogIntent {
    pub fn intent_for_key(key: KeyCode) -> Option<DialogIntent> {
        match key {
            KeyCode::Ok => Some(DialogIntent::Confirm),
            KeyCode::Back => Some(DialogIntent::Back),
            _ => None,
        }
    }
}

pub fn reduce(
    state: &mut GameState,
    player: &mut PlayerState,
    data: &GameData,
    intent: DialogIntent,
) {
    match intent {
        DialogIntent::Confirm => {
            if let GameState::Dialog(ref dialog_state) = *state
                && let Some(dialog) = data.find_dialog(&dialog_state.dialog_id)
                && let Some(action) = dialog
                    .lines
                    .get(dialog_state.current_line)
                    .and_then(|line| line.action.as_ref())
                    .cloned()
                && let Some(new_state) = npc::reduce(
                    player,
                    data,
                    NpcIntent::ProcessDialogAction { action: &action },
                )
            {
                *state = new_state;
            }

            if matches!(*state, GameState::Shop(_)) {
                return;
            }

            if let GameState::Dialog(ref mut dialog_state) = *state
                && let Some(dialog) = data.find_dialog(&dialog_state.dialog_id)
                && !(if dialog_state.current_line + 1 < dialog.lines.len() {
                    dialog_state.current_line += 1;
                    true
                } else {
                    false
                })
            {
                *state = GameState::Explore;
            }
        }
        DialogIntent::Back => *state = GameState::Explore,
    }
}

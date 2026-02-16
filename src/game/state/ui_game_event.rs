use anyhow::Result;

use crate::game::{
    DialogState, GameEvent, GameState, MenuState, SessionState, UiState, has_save_data,
};

impl UiState {
    pub fn apply_game_event(
        &mut self,
        data: &crate::game::GameData,
        state: &mut GameState,
        session: &mut Option<SessionState>,
        event: &GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::StartNewGame | GameEvent::ContinueGame => {
                *self = UiState::default();
                if matches!(state, GameState::Dialog)
                    && let Some(dialog_state) = intro_dialog_state(data)
                {
                    self.dialog.set(Some(dialog_state));
                }
            }
            GameEvent::Menu(event) => match event {
                crate::game::MenuEvent::None => {}
                crate::game::MenuEvent::SetSelected(selected) => self.menu.set_selected(*selected),
                crate::game::MenuEvent::Action(_) => {}
            },
            GameEvent::PauseMenu(event) => match event {
                crate::game::PauseMenuEvent::None => {}
                crate::game::PauseMenuEvent::SetSelected(selected) => {
                    self.pause_menu.set_selected(*selected)
                }
                crate::game::PauseMenuEvent::OpenInventory => {
                    self.inventory.reset();
                    state.transition_to(session, GameState::Inventory);
                }
                crate::game::PauseMenuEvent::OpenStats => {
                    state.transition_to(session, GameState::Stats)
                }
                crate::game::PauseMenuEvent::OpenQuestLog => {
                    state.transition_to(session, GameState::QuestLog)
                }
                crate::game::PauseMenuEvent::SaveAndReturnExplore => {
                    self.shop.reset();
                    state.transition_to(session, GameState::Explore);
                }
                crate::game::PauseMenuEvent::BackToExplore => {
                    state.transition_to(session, GameState::Explore)
                }
            },
            GameEvent::OpenPauseMenu => {
                self.pause_menu.reset();
                state.transition_to(session, GameState::PauseMenu);
            }
            GameEvent::OpenMenuFromExplore => {
                self.menu.set_menu(MenuState::new(has_save_data()));
                state.transition_to(session, GameState::Menu);
            }
            GameEvent::Inventory(event) => match event {
                crate::game::InventoryEvent::None => {}
                crate::game::InventoryEvent::SetSelected(selected) => {
                    self.inventory.set_selected(*selected)
                }
                crate::game::InventoryEvent::UseSelected(_) => {}
                crate::game::InventoryEvent::CloseToExplore => {
                    state.transition_to(session, GameState::Explore)
                }
            },
            GameEvent::ApplyDialogTransition(transition) => match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = self.dialog.state.as_mut() {
                        dialog_state.current_line = *line;
                    }
                    state.transition_to(session, GameState::Dialog);
                }
                crate::game::DialogTransition::CloseToExplore => {
                    self.dialog.close();
                    state.transition_to(session, GameState::Explore);
                }
            },
            GameEvent::Shop(event) => match event {
                crate::game::ShopEvent::BuyItem(_) => {}
                crate::game::ShopEvent::SellSelected(index) => {
                    if let Some(s) = session.as_ref() {
                        let len_after = s.player.inventory.len();
                        let current_selected = self.shop.selected;
                        if *index >= len_after
                            && current_selected >= len_after
                            && current_selected > 0
                        {
                            self.shop.set_selected(current_selected - 1);
                        }
                    }
                }
                crate::game::ShopEvent::CloseToExplore => {
                    state.transition_to(session, GameState::Explore)
                }
            },
            GameEvent::OpenDialogState(dialog_state) => {
                self.dialog.open(dialog_state.clone());
                state.transition_to(session, GameState::Dialog);
            }
            _ => {}
        }
        Ok(())
    }
}

fn intro_dialog_state(data: &crate::game::GameData) -> Option<DialogState> {
    let (dialog_id, npc_name) = data.newgame.intro_dialog.as_ref()?;
    let dialog = data.find_dialog(dialog_id)?;
    Some(DialogState::from_dialog(npc_name.clone(), dialog))
}

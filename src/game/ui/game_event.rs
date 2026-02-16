use anyhow::Result;

use crate::game::{GameEvent, MenuState, SessionState, UiState, has_save_data};

impl UiState {
    pub fn apply_game_event(
        &mut self,
        session: Option<&SessionState>,
        event: &GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Lifecycle(crate::game::LifecycleEvent::ResetUi) => {
                *self = UiState::default();
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
                }
                crate::game::PauseMenuEvent::OpenStats => {}
                crate::game::PauseMenuEvent::OpenQuestLog => {}
                crate::game::PauseMenuEvent::SaveAndReturnExplore => {
                    self.shop.reset();
                }
                crate::game::PauseMenuEvent::BackToExplore => {}
            },
            GameEvent::OpenPauseMenu => {
                self.pause_menu.reset();
            }
            GameEvent::OpenMenuFromExplore => {
                self.menu.set_menu(MenuState::new(has_save_data()));
            }
            GameEvent::Loading(crate::game::LoadingEvent::Loaded)
            | GameEvent::Transition(crate::game::TransitionEvent::ToMenuFromGameOver) => {
                self.menu.set_menu(MenuState::new(has_save_data()));
            }
            GameEvent::Inventory(event) => match event {
                crate::game::InventoryEvent::None => {}
                crate::game::InventoryEvent::SetSelected(selected) => {
                    self.inventory.set_selected(*selected)
                }
                crate::game::InventoryEvent::UseSelected(_) => {}
                crate::game::InventoryEvent::CloseToExplore => {}
            },
            GameEvent::ApplyDialogTransition(transition) => match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = self.dialog.state.as_mut() {
                        dialog_state.current_line = *line;
                    }
                }
                crate::game::DialogTransition::CloseToExplore => {
                    self.dialog.close();
                }
            },
            GameEvent::Shop(event) => match event {
                crate::game::ShopEvent::BuyItem(_) => {}
                crate::game::ShopEvent::SellSelected(index) => {
                    if let Some(s) = session.as_ref() {
                        let len_after = s.leader.inventory.len();
                        let current_selected = self.shop.selected;
                        if *index >= len_after
                            && current_selected >= len_after
                            && current_selected > 0
                        {
                            self.shop.set_selected(current_selected - 1);
                        }
                    }
                }
                crate::game::ShopEvent::CloseToExplore => {}
            },
            GameEvent::OpenDialogState(dialog_state) => {
                self.dialog.open(dialog_state.clone());
            }
            GameEvent::OpenShopState(shop_state) => {
                self.shop.open((**shop_state).clone());
            }
            _ => {}
        }
        Ok(())
    }
}

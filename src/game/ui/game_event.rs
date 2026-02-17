use anyhow::Result;

use crate::game::{
    GameEvent, GameEventKind, GameEventSubscriber, MenuState, UiState, WorldState, has_save_data,
};

impl UiState {
    pub fn apply_game_event(
        &mut self,
        session: Option<&WorldState>,
        event: &GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Lifecycle(crate::game::LifecycleEvent::ResetUi) => {
                *self = UiState::default();
            }
            GameEvent::Loading(crate::game::LoadingEvent::Loaded)
            | GameEvent::Transition(crate::game::TransitionEvent::ToMenu)
            | GameEvent::Transition(crate::game::TransitionEvent::ToMenuFromGameOver) => {
                self.menu.state = MenuState::new(has_save_data());
                self.menu.selected = 0;
            }
            GameEvent::Transition(crate::game::TransitionEvent::ToPauseMenu) => {
                self.pause_menu.selected = 0;
            }
            GameEvent::Transition(crate::game::TransitionEvent::ToInventory) => {
                self.inventory.selected = 0;
            }
            GameEvent::ApplyDialogTransition(transition) => match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = self.dialog.state.as_mut() {
                        dialog_state.current_line = *line;
                    }
                }
                crate::game::DialogTransition::CloseToExplore => {
                    self.dialog.state = None;
                }
            },
            GameEvent::ShopBuyItem(_) => {}
            GameEvent::ShopSellSelected(index) => {
                if let Some(s) = session.as_ref() {
                    let len_after = s.leader.inventory.len();
                    let current_selected = self.shop.selected;
                    if *index >= len_after && current_selected >= len_after && current_selected > 0
                    {
                        self.shop.selected = current_selected - 1;
                    }
                }
            }
            GameEvent::OpenDialogState(dialog_state) => {
                self.dialog.state = Some(dialog_state.clone());
            }
            GameEvent::OpenShopState(shop_state) => {
                self.shop.state = Some((**shop_state).clone());
                self.shop.mode = crate::game::ShopMode::Select;
                self.shop.selected = 0;
            }
            _ => {}
        }
        Ok(())
    }
}

impl GameEventSubscriber for UiState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(
            kind,
            GameEventKind::Lifecycle
                | GameEventKind::Loading
                | GameEventKind::Transition
                | GameEventKind::ApplyDialogTransition
                | GameEventKind::ShopSellSelected
                | GameEventKind::OpenDialogState
                | GameEventKind::OpenShopState
        )
    }
}

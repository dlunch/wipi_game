use anyhow::Result;

use super::state::MenuState;
use crate::game::save::has_save_data;
use crate::game::{GameEvent, GameEventKind, GameEventSubscriber, UiState};

impl UiState {
    pub fn apply_game_event(&mut self, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Lifecycle(crate::game::LifecycleEvent::ResetUi) => {
                *self = UiState::default();
            }
            GameEvent::Loading(crate::game::LoadingEvent::Loaded)
            | GameEvent::Transition(crate::game::TransitionEvent::ToMenu) => {
                self.menu.state = MenuState::new(has_save_data());
                self.menu.selected = 0;
            }
            GameEvent::Transition(crate::game::TransitionEvent::ToPauseMenu) => {
                self.pause_menu.selected = 0;
            }
            GameEvent::Transition(crate::game::TransitionEvent::ToInventory) => {
                self.inventory.selected = 0;
            }
            GameEvent::Transition(crate::game::TransitionEvent::ToQuestLog) => {
                self.quest_log.selected = 0;
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
            GameEvent::ShopSellSelected(index) => {
                if *index <= self.shop.selected && self.shop.selected > 0 {
                    self.shop.selected -= 1;
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
            GameEvent::World(crate::game::WorldEvent::SetQuestRewarded { quest_id, rewarded })
                if *rewarded && self.quest_log.tracked_quest_id.as_deref() == Some(quest_id) =>
            {
                self.quest_log.tracked_quest_id = None;
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
                | GameEventKind::World
                | GameEventKind::ApplyDialogTransition
                | GameEventKind::ShopSellSelected
                | GameEventKind::OpenDialogState
                | GameEventKind::OpenShopState
        )
    }
}

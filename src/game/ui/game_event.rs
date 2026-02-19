use anyhow::Result;

use super::state::{MenuState, ShopMode, UiState};
use crate::game::{
    game_event::{GameEvent, GameEventKind, GameEventSubscriber, TransitionEvent, WorldEvent},
    systems::lifecycle::{LifecycleEvent, LoadingEvent},
    ui::state::DialogTransition,
};

impl UiState {
    pub fn apply_game_event(&mut self, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Lifecycle(LifecycleEvent::ResetUi) => {
                self.reset();
            }
            GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(has_save)) => {
                self.menu.state = MenuState::new(*has_save);
                self.menu.selected = 0;
            }
            GameEvent::Loading(LoadingEvent::Loaded)
            | GameEvent::Transition(TransitionEvent::ToMenu) => {
                // Menu content is configured by Lifecycle::SetMenuHasSaveData.
                self.menu.selected = 0;
            }
            GameEvent::Transition(TransitionEvent::ToPauseMenu) => {
                self.pause_menu.selected = 0;
            }
            GameEvent::Transition(TransitionEvent::ToInventory) => {
                self.inventory.selected = 0;
            }
            GameEvent::Transition(TransitionEvent::ToQuestLog) => {
                self.quest_log.selected = 0;
            }
            GameEvent::ApplyDialogTransition(transition) => match transition {
                DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = self.dialog.state.as_mut() {
                        dialog_state.current_line = *line;
                    }
                }
                DialogTransition::CloseToExplore => {
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
                self.shop.mode = ShopMode::Select;
                self.shop.selected = 0;
            }
            GameEvent::World(WorldEvent::SetQuestRewarded { quest_id, rewarded })
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

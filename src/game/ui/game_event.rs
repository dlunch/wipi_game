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
            GameEvent::ShopSellItem(_) => {
                let sell_len = self.shop.sell_item_ids.len();
                if sell_len == 0 {
                    self.shop.selected = 0;
                } else if self.shop.selected >= sell_len {
                    self.shop.selected = sell_len - 1;
                }
            }
            GameEvent::OpenDialogState(dialog_state) => {
                self.dialog.state = Some(dialog_state.clone());
            }
            GameEvent::OpenShopById(shop_id) => {
                self.shop.shop_id = Some(shop_id.clone());
                self.shop.buy_item_ids.clear();
                self.shop.sell_item_ids.clear();
                self.shop.mode = ShopMode::Select;
                self.shop.selected = 0;
            }
            GameEvent::SetShopBuyItemIds(item_ids) => {
                self.shop.buy_item_ids = item_ids.clone();
            }
            GameEvent::SetShopSellItemIds(item_ids) => {
                self.shop.sell_item_ids = item_ids.clone();
                if self.shop.sell_item_ids.is_empty() {
                    self.shop.selected = 0;
                } else if self.shop.selected >= self.shop.sell_item_ids.len() {
                    self.shop.selected = self.shop.sell_item_ids.len() - 1;
                }
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
                | GameEventKind::ShopSellItem
                | GameEventKind::OpenDialogState
                | GameEventKind::OpenShopById
                | GameEventKind::SetShopBuyItemIds
                | GameEventKind::SetShopSellItemIds
        )
    }
}

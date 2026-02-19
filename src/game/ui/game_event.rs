use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use super::state::{DialogState, MenuState, ShopMode, UiState};
use crate::{
    data::{DialogCondition, DialogLine},
    game::{
        game_data::GameData,
        game_event::{GameEvent, GameEventKind, GameEventSubscriber, TransitionEvent, WorldEvent},
        systems::lifecycle::{LifecycleEvent, LoadingEvent},
        world::WorldState,
    },
};

impl UiState {
    pub fn apply_game_event(
        &mut self,
        data: &GameData,
        world: Option<&WorldState>,
        event: &GameEvent,
    ) -> Result<()> {
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
            GameEvent::Transition(TransitionEvent::ToExplore) => {
                self.dialog.state = None;
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
            GameEvent::ShopSellItem(_) => {
                let sell_len = self.shop.sell_item_ids.len();
                if sell_len == 0 {
                    self.shop.selected = 0;
                } else if self.shop.selected >= sell_len {
                    self.shop.selected = sell_len - 1;
                }
            }
            GameEvent::OpenDialog { dialog_id, npc_id } => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                let lines = visible_dialog_lines(world, data, *dialog_id)?;
                if lines.is_empty() {
                    self.dialog.state = None;
                } else {
                    let npc_name = data.find_npc(*npc_id)?.name.clone();
                    self.dialog.state = Some(DialogState::new(npc_name, lines));
                }
            }
            GameEvent::OpenShopById(shop_id) => {
                self.shop.shop_id = Some(*shop_id);
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
                if *rewarded && self.quest_log.tracked_quest_id == Some(*quest_id) =>
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
                | GameEventKind::ShopSellItem
                | GameEventKind::OpenDialog
                | GameEventKind::OpenShopById
                | GameEventKind::SetShopBuyItemIds
                | GameEventKind::SetShopSellItemIds
        )
    }
}

fn visible_dialog_lines(
    world: &WorldState,
    data: &GameData,
    dialog_id: u32,
) -> Result<Vec<DialogLine>> {
    let leader_id = world.leader_id()?;
    let dialog = data.find_dialog(dialog_id)?;
    let mut lines = Vec::with_capacity(dialog.lines.len());
    for line in &dialog.lines {
        let visible = match &line.condition {
            None => true,
            Some(DialogCondition::HasQuest(id)) => world.has_quest(*id),
            Some(DialogCondition::QuestComplete(id)) => world.is_quest_complete(*id),
            Some(DialogCondition::HasItem(id)) => world.has_item(leader_id, *id)?,
            Some(DialogCondition::HasGold(amount)) => world.gold_amount(leader_id)? >= *amount,
        };
        if visible {
            lines.push(line.clone());
        }
    }
    Ok(lines)
}

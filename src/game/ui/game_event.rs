use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use super::state::{DialogState, MenuState, ShopMode, UiState};
use crate::{
    data::{DialogAction, DialogCondition, DialogId},
    game::{
        game_data::GameData,
        game_event::{
            GameEvent, GameEventKind, GameEventSubscriber, QuestFlag, ShopItemListKind,
            TransitionEvent, WorldEvent,
        },
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
                let sell_len = self.shop.sell_items.len();
                if sell_len == 0 {
                    self.shop.selected = 0;
                } else if self.shop.selected >= sell_len {
                    self.shop.selected = sell_len - 1;
                }
            }
            GameEvent::OpenDialog { dialog_id, npc_id } => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                let (visible_line_indices, visible_actions) =
                    visible_dialog_lines(world, data, *dialog_id)?;
                if visible_line_indices.is_empty() {
                    self.dialog.state = None;
                } else {
                    self.dialog.state = Some(DialogState::new(
                        *dialog_id,
                        *npc_id,
                        visible_line_indices,
                        visible_actions,
                    ));
                }
            }
            GameEvent::OpenShopById(shop_id) => {
                self.shop.shop_id = Some(*shop_id);
                self.shop.buy_items.clear();
                self.shop.sell_items.clear();
                self.shop.mode = ShopMode::Select;
                self.shop.selected = 0;
            }
            GameEvent::SetShopItems { list, items } => match list {
                ShopItemListKind::Buy => {
                    self.shop.buy_items = items.clone();
                }
                ShopItemListKind::Sell => {
                    self.shop.sell_items = items.clone();
                    if self.shop.sell_items.is_empty() {
                        self.shop.selected = 0;
                    } else if self.shop.selected >= self.shop.sell_items.len() {
                        self.shop.selected = self.shop.sell_items.len() - 1;
                    }
                }
            },
            GameEvent::World(WorldEvent::SetQuestFlag {
                quest_id,
                flag: QuestFlag::Rewarded,
                value,
            }) if *value && self.quest_log.tracked_quest_id == Some(*quest_id) => {
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
                | GameEventKind::SetShopItems
        )
    }
}

fn visible_dialog_lines(
    world: &WorldState,
    data: &GameData,
    dialog_id: DialogId,
) -> Result<(Vec<usize>, Vec<Option<DialogAction>>)> {
    let leader_id = world.leader_id()?;
    let dialog = data.find_dialog(dialog_id)?;
    let mut indices = Vec::with_capacity(dialog.lines.len());
    let mut actions = Vec::with_capacity(dialog.lines.len());
    for (index, line) in dialog.lines.iter().enumerate() {
        let visible = match &line.condition {
            None => true,
            Some(DialogCondition::HasQuest(id)) => world.has_quest(*id),
            Some(DialogCondition::QuestComplete(id)) => world.is_quest_complete(*id),
            Some(DialogCondition::HasItem(id)) => world.has_item(leader_id, *id)?,
            Some(DialogCondition::HasGold(amount)) => world.gold_amount(leader_id)? >= *amount,
        };
        if visible {
            indices.push(index);
            actions.push(line.action.clone());
        }
    }
    Ok((indices, actions))
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec};

    use anyhow::{Result, anyhow};

    use super::UiState;
    use crate::game::{
        game_data::{GameData, load_step as load_game_data_step},
        game_event::{EntityEvent, GameEvent, ShopItemEntry, ShopItemListKind, WorldEvent},
        state::EntityKind,
        world::WorldState,
    };

    fn load_dialog_data() -> Result<GameData> {
        let mut data = GameData::new(|path| {
            if path == "data/dialogs.dat" {
                return Ok(b"@DIALOG:1\nHello\nOPEN_SHOP=9:Welcome\n@END\n".to_vec());
            }
            Err(anyhow!("unexpected path: {}", path))
        });
        load_game_data_step(&mut data, 4)?;
        Ok(data)
    }

    fn build_world_with_leader() -> Result<WorldState> {
        let mut world = WorldState::empty();
        let data = GameData::new(|path| Err(anyhow!("unexpected path: {}", path)));
        world.apply_event(&data, &GameEvent::World(WorldEvent::CreateWorld))?;
        world.apply_event(&data, &GameEvent::Entity(EntityEvent::SetLeaderEntity(1)))?;
        world.apply_event(
            &data,
            &GameEvent::Entity(EntityEvent::CreateEntity {
                entity_id: 1,
                kind: EntityKind::Player,
                name: String::from("Hero"),
                source_enemy_id: None,
            }),
        )?;
        Ok(world)
    }

    #[test]
    fn open_dialog_stores_indices_and_actions() -> Result<()> {
        let mut ui = UiState::default();
        let data = load_dialog_data()?;
        let world = build_world_with_leader()?;

        ui.apply_game_event(
            &data,
            Some(&world),
            &GameEvent::OpenDialog {
                dialog_id: 1,
                npc_id: 42,
            },
        )?;

        let dialog_state = ui
            .dialog
            .state
            .as_ref()
            .ok_or_else(|| anyhow!("dialog state should exist"))?;
        assert_eq!(dialog_state.dialog_id, 1);
        assert_eq!(dialog_state.npc_id, 42);
        assert_eq!(dialog_state.visible_line_indices, vec![0, 1]);
        assert_eq!(dialog_state.visible_actions.len(), 2);
        assert!(dialog_state.visible_actions[0].is_none());
        assert!(dialog_state.visible_actions[1].is_some());
        Ok(())
    }

    #[test]
    fn set_shop_items_updates_buy_and_sell_lists() -> Result<()> {
        let mut ui = UiState::default();
        let data = load_dialog_data()?;
        let world = build_world_with_leader()?;

        ui.apply_game_event(
            &data,
            Some(&world),
            &GameEvent::SetShopItems {
                list: ShopItemListKind::Buy,
                items: vec![
                    ShopItemEntry {
                        item_id: 10,
                        amount: 1,
                    },
                    ShopItemEntry {
                        item_id: 11,
                        amount: 1,
                    },
                ],
            },
        )?;
        ui.shop.selected = 3;
        ui.apply_game_event(
            &data,
            Some(&world),
            &GameEvent::SetShopItems {
                list: ShopItemListKind::Sell,
                items: vec![ShopItemEntry {
                    item_id: 12,
                    amount: 4,
                }],
            },
        )?;

        assert_eq!(ui.shop.buy_items.len(), 2);
        assert_eq!(ui.shop.sell_items.len(), 1);
        assert_eq!(ui.shop.sell_items[0].amount, 4);
        assert_eq!(ui.shop.selected, 0);
        Ok(())
    }
}

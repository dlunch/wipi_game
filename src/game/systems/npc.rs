use super::combat::Direction;
use crate::data::{Dialog, DialogAction, DialogCondition, NpcType};
use crate::game::{self, DialogState, GameData, GameState, PlayerIntent, PlayerState, ShopState};

#[derive(Debug)]
pub enum NpcIntent<'a> {
    Interact { facing: Direction },
    ProcessDialogAction { action: &'a DialogAction },
}

pub fn reduce(
    player: &mut PlayerState,
    data: &GameData,
    intent: NpcIntent<'_>,
) -> Option<GameState> {
    match intent {
        NpcIntent::Interact { facing } => try_interact(player, data, facing),
        NpcIntent::ProcessDialogAction { action } => process_action(player, data, action),
    }
}

pub fn try_interact(
    player: &mut PlayerState,
    data: &GameData,
    facing: Direction,
) -> Option<GameState> {
    let (target_x, target_y) = facing.apply(player.x, player.y);

    let npc = data.find_npc_at(&player.current_map_id, target_x, target_y)?;

    match npc.npc_type {
        NpcType::Healer => {
            let _ = game::player::reduce(player, PlayerIntent::FullHeal);

            if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
                let filtered = filter_lines(player, dialog);
                if !filtered.lines.is_empty() {
                    return Some(GameState::Dialog(DialogState::new(
                        npc.name.clone(),
                        &filtered,
                    )));
                }
            }
        }
        NpcType::ShopKeeper => {
            let shop = npc
                .shop_id
                .as_ref()
                .and_then(|sid| data.find_shop(sid))
                .or_else(|| data.shops.first())
                .cloned();

            if let Some(shop) = shop {
                let shop_items = data.get_shop_items(&shop);
                return Some(GameState::Shop(ShopState::new(shop, shop_items)));
            }
        }
        NpcType::QuestGiver | NpcType::Villager => {}
    }

    if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
        let filtered = filter_lines(player, dialog);
        if !filtered.lines.is_empty() {
            return Some(GameState::Dialog(DialogState::new(
                npc.name.clone(),
                &filtered,
            )));
        }
    }

    None
}

pub fn filter_lines(player: &PlayerState, dialog: &Dialog) -> Dialog {
    let filtered = dialog
        .lines
        .iter()
        .filter(|line| match &line.condition {
            None => true,
            Some(DialogCondition::HasQuest(id)) => player.has_quest(id),
            Some(DialogCondition::QuestComplete(id)) => player.is_quest_complete(id),
            Some(DialogCondition::HasItem(id)) => player.has_item(id),
            Some(DialogCondition::HasGold(amount)) => player.stats.gold >= *amount,
        })
        .cloned()
        .collect();

    Dialog {
        id: dialog.id.clone(),
        lines: filtered,
    }
}

pub fn process_action(
    player: &mut PlayerState,
    data: &GameData,
    action: &DialogAction,
) -> Option<GameState> {
    match action {
        DialogAction::GiveQuest(id) => {
            let _ = game::player::reduce(player, PlayerIntent::AddQuest(id.clone()));
        }
        DialogAction::CompleteQuest(id) => {
            let can_reward = player
                .quests
                .iter()
                .any(|q| q.quest_id == *id && q.completed && !q.rewarded);

            if can_reward && let Some(quest) = data.find_quest(id) {
                let _ = game::player::reduce(player, PlayerIntent::AddExp(quest.reward_exp));
                let _ = game::player::reduce(player, PlayerIntent::AddGold(quest.reward_gold));

                if let Some(item_id) = &quest.reward_item
                    && let Some(item) = data.find_item(item_id).cloned()
                {
                    let _ = game::player::reduce(player, PlayerIntent::AddItem(item));
                }

                let _ = game::player::reduce(player, PlayerIntent::MarkQuestRewarded(id.clone()));
            }
        }
        DialogAction::GiveItem(id) => {
            if let Some(item) = data.find_item(id).cloned() {
                let _ = game::player::reduce(player, PlayerIntent::AddItem(item));
            }
        }
        DialogAction::TakeItem(id) => {
            let _ = game::player::reduce(player, PlayerIntent::RemoveItem(id.clone()));
        }
        DialogAction::GiveGold(amount) => {
            let _ = game::player::reduce(player, PlayerIntent::AddGold(*amount));
        }
        DialogAction::TakeGold(amount) => {
            let _ = game::player::reduce(player, PlayerIntent::AddGold(-*amount));
        }
        DialogAction::OpenShop(id) => {
            if let Some(shop) = data.find_shop(id).cloned() {
                let shop_items = data.get_shop_items(&shop);
                return Some(GameState::Shop(ShopState::new(shop, shop_items)));
            }
        }
        DialogAction::Heal => {
            let _ = game::player::reduce(player, PlayerIntent::FullHeal);
        }
    }
    None
}

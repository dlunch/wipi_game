use super::combat::Direction;
use crate::data::{Dialog, DialogAction, DialogCondition, NpcType};
use crate::game::{DialogState, GameData, GameState, PlayerState, ShopState};

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
            player.stats.current_hp = player.stats.max_hp;
            player.stats.current_mp = player.stats.max_mp;

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
            player.add_quest(id);
        }
        DialogAction::CompleteQuest(id) => {
            let can_reward = player
                .quests
                .iter()
                .any(|q| q.quest_id == *id && q.completed && !q.rewarded);

            if can_reward && let Some(quest) = data.find_quest(id) {
                player.stats.add_exp(quest.reward_exp);
                player.stats.gold += quest.reward_gold;

                if let Some(item_id) = &quest.reward_item
                    && let Some(item) = data.find_item(item_id).cloned()
                {
                    player.add_item(item);
                }

                player.mark_quest_rewarded(id);
            }
        }
        DialogAction::GiveItem(id) => {
            if let Some(item) = data.find_item(id).cloned() {
                player.add_item(item);
            }
        }
        DialogAction::TakeItem(id) => {
            player.remove_item(id);
        }
        DialogAction::GiveGold(amount) => {
            player.stats.gold += amount;
        }
        DialogAction::TakeGold(amount) => {
            player.stats.gold = (player.stats.gold - amount).max(0);
        }
        DialogAction::OpenShop(id) => {
            if let Some(shop) = data.find_shop(id).cloned() {
                let shop_items = data.get_shop_items(&shop);
                return Some(GameState::Shop(ShopState::new(shop, shop_items)));
            }
        }
        DialogAction::Heal => {
            player.stats.current_hp = player.stats.max_hp;
            player.stats.current_mp = player.stats.max_mp;
        }
    }
    None
}

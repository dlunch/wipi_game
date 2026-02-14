use alloc::vec::Vec;

use crate::data::QuestType;
use crate::game::{self, GameData, PlayerIntent, PlayerState};

pub fn on_enemy_killed(player: &mut PlayerState, data: &GameData, enemy_id: &str) {
    let mut updates = Vec::new();
    for progress in &player.quests {
        if progress.completed || progress.rewarded {
            continue;
        }

        if let Some(quest) = data.find_quest(&progress.quest_id)
            && quest.quest_type == QuestType::Kill
            && quest.target_id == enemy_id
        {
            updates.push((progress.quest_id.clone(), quest.target_count));
        }
    }

    for (quest_id, target_count) in updates {
        let _ = game::player::reduce(
            player,
            PlayerIntent::UpdateQuestProgress {
                quest_id,
                target_count,
            },
        );
    }
}

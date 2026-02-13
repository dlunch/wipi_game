use crate::data::QuestType;
use crate::game::{GameData, Player};

pub fn on_enemy_killed(player: &mut Player, data: &GameData, enemy_id: &str) {
    for progress in &mut player.quests {
        if progress.completed || progress.rewarded {
            continue;
        }

        if let Some(quest) = data.find_quest(&progress.quest_id)
            && quest.quest_type == QuestType::Kill
            && quest.target_id == enemy_id
        {
            progress.current_count += 1;
            if progress.current_count >= quest.target_count {
                progress.completed = true;
            }
        }
    }
}

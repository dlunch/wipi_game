use alloc::vec::Vec;

use crate::data::QuestType;
use crate::game::{self, GameData, PlayerIntent, PlayerState};

pub enum QuestIntent<'a> {
    EnemyKilled { enemy_id: &'a str },
}

pub fn reduce(player: &mut PlayerState, data: &GameData, intent: QuestIntent<'_>) {
    match intent {
        QuestIntent::EnemyKilled { enemy_id } => {
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
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use crate::data::{Quest, QuestProgress, QuestType};
    use crate::game::GameData;

    use super::{QuestIntent, reduce};

    fn make_quest(id: &str, target_id: &str, target_count: i32) -> Quest {
        Quest {
            id: String::from(id),
            name: String::from("Test Quest"),
            description: String::from("Defeat enemies"),
            quest_type: QuestType::Kill,
            target_id: String::from(target_id),
            target_count,
            reward_exp: 10,
            reward_gold: 5,
            reward_item: None,
        }
    }

    fn make_player_with_quest(
        quest_id: &str,
        current_count: i32,
        completed: bool,
        rewarded: bool,
    ) -> crate::game::PlayerState {
        let mut player = crate::game::PlayerState::new(String::from("H"), "v");
        player.quests.push(QuestProgress {
            quest_id: String::from(quest_id),
            current_count,
            completed,
            rewarded,
        });
        player
    }

    fn make_game_data_with_quest(quest: Quest) -> GameData {
        let mut data = GameData::default();
        data.quests = vec![quest];
        data
    }

    #[test]
    fn enemy_killed_matching_kill_quest_increments_current_count() {
        let mut player = make_player_with_quest("q1", 0, false, false);
        let data = make_game_data_with_quest(make_quest("q1", "slime", 3));

        reduce(
            &mut player,
            &data,
            QuestIntent::EnemyKilled { enemy_id: "slime" },
        );

        assert_eq!(player.quests[0].current_count, 1);
        assert!(!player.quests[0].completed);
    }

    #[test]
    fn enemy_killed_reaching_target_count_sets_completed_true() {
        let mut player = make_player_with_quest("q1", 1, false, false);
        let data = make_game_data_with_quest(make_quest("q1", "slime", 2));

        reduce(
            &mut player,
            &data,
            QuestIntent::EnemyKilled { enemy_id: "slime" },
        );

        assert_eq!(player.quests[0].current_count, 2);
        assert!(player.quests[0].completed);
    }

    #[test]
    fn enemy_killed_with_non_matching_enemy_id_no_change() {
        let mut player = make_player_with_quest("q1", 1, false, false);
        let data = make_game_data_with_quest(make_quest("q1", "slime", 3));

        reduce(
            &mut player,
            &data,
            QuestIntent::EnemyKilled { enemy_id: "goblin" },
        );

        assert_eq!(player.quests[0].current_count, 1);
        assert!(!player.quests[0].completed);
    }

    #[test]
    fn enemy_killed_with_already_completed_quest_no_change() {
        let mut player = make_player_with_quest("q1", 3, true, false);
        let data = make_game_data_with_quest(make_quest("q1", "slime", 3));

        reduce(
            &mut player,
            &data,
            QuestIntent::EnemyKilled { enemy_id: "slime" },
        );

        assert_eq!(player.quests[0].current_count, 3);
        assert!(player.quests[0].completed);
    }

    #[test]
    fn enemy_killed_with_already_rewarded_quest_no_change() {
        let mut player = make_player_with_quest("q1", 2, false, true);
        let data = make_game_data_with_quest(make_quest("q1", "slime", 3));

        reduce(
            &mut player,
            &data,
            QuestIntent::EnemyKilled { enemy_id: "slime" },
        );

        assert_eq!(player.quests[0].current_count, 2);
        assert!(!player.quests[0].completed);
        assert!(player.quests[0].rewarded);
    }

    #[test]
    fn enemy_killed_with_no_quests_no_change() {
        let mut player = crate::game::PlayerState::new(String::from("H"), "v");
        let data = make_game_data_with_quest(make_quest("q1", "slime", 3));

        reduce(
            &mut player,
            &data,
            QuestIntent::EnemyKilled { enemy_id: "slime" },
        );

        assert!(player.quests.is_empty());
    }

    #[test]
    fn multiple_quests_tracking_same_enemy_both_update() {
        let mut player = make_player_with_quest("q1", 0, false, false);
        player.quests.push(QuestProgress {
            quest_id: String::from("q2"),
            current_count: 1,
            completed: false,
            rewarded: false,
        });

        let mut data = make_game_data_with_quest(make_quest("q1", "slime", 3));
        data.quests.push(make_quest("q2", "slime", 2));

        reduce(
            &mut player,
            &data,
            QuestIntent::EnemyKilled { enemy_id: "slime" },
        );

        let counts: Vec<i32> = player.quests.iter().map(|q| q.current_count).collect();
        assert_eq!(counts, vec![1, 2]);
        assert!(!player.quests[0].completed);
        assert!(player.quests[1].completed);
    }
}

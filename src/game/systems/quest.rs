use crate::game::{GameData, PlayerState};

pub enum QuestIntent<'a> {
    EnemyKilled { enemy_id: &'a str },
}

pub fn apply(player: &mut PlayerState, data: &GameData, intent: QuestIntent<'_>) {
    match intent {
        QuestIntent::EnemyKilled { enemy_id } => player.apply_quest_kill(data, enemy_id),
    }
}

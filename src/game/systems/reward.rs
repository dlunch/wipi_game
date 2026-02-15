use crate::game::combat::KillReward;
use crate::game::{self, GameData, PlayerIntent, PlayerState, QuestIntent};

pub fn apply_kill_reward(player: &mut PlayerState, data: &GameData, reward: &KillReward) {
    let _ = game::player::reduce(player, PlayerIntent::AddExp(reward.exp));
    let _ = game::player::reduce(player, PlayerIntent::AddGold(reward.gold));
    game::quest::reduce(
        player,
        data,
        QuestIntent::EnemyKilled {
            enemy_id: &reward.enemy_id,
        },
    );
}

pub fn apply_kill_rewards(player: &mut PlayerState, data: &GameData, rewards: &[KillReward]) {
    for reward in rewards {
        apply_kill_reward(player, data, reward);
    }
}

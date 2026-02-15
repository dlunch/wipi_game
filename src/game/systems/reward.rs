use crate::game::PlayerState;
use crate::game::combat::KillReward;

pub fn apply_kill_reward(player: &mut PlayerState, reward: &KillReward) {
    player.stats.add_exp(reward.exp);
    player.stats.gold = (player.stats.gold + reward.gold).max(0);
}

pub fn apply_kill_rewards(player: &mut PlayerState, rewards: &[KillReward]) {
    for reward in rewards {
        apply_kill_reward(player, reward);
    }
}

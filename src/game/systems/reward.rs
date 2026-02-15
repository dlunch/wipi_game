use crate::game::{KillReward, PlayerState};

pub fn apply_kill_reward(player: &mut PlayerState, reward: &KillReward) {
    player.apply_kill_reward(reward);
}

pub fn apply_kill_rewards(player: &mut PlayerState, rewards: &[KillReward]) {
    player.apply_kill_rewards(rewards);
}

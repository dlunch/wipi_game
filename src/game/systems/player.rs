use crate::game::{PlayerAction, PlayerEvent, PlayerState};

pub fn apply(player: &mut PlayerState, action: PlayerAction) -> PlayerEvent {
    player.apply(action)
}

pub fn can_use_skill(
    player: &PlayerState,
    cooldowns: &[u32; 3],
    slot: usize,
    mp_cost: i32,
) -> bool {
    player.can_use_skill(cooldowns, slot, mp_cost)
}

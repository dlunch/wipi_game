use crate::game::{CombatState, InventoryState, MovementState, PlayerState};

pub struct SessionState {
    pub player: PlayerState,
    pub combat: CombatState,
    pub movement: MovementState,
    pub inventory: InventoryState,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
}

use crate::game::{CombatState, InventoryState, MovementState, PlayerState};

pub struct SessionState {
    pub player: PlayerState,
    pub combat: CombatState,
    pub movement: MovementState,
    pub inventory: InventoryState,
}

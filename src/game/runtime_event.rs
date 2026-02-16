use crate::data::Direction;
use crate::game::state::CombatState;

pub enum CombatRuntimeEvent {
    Tick {
        next_state: CombatState,
        next_skill_cooldowns: [u32; 3],
        next_mp_regen_timer: u32,
        recover_mp: i32,
        damage_taken: i32,
    },
}

pub enum RuntimeEvent {
    Loading(crate::game::LoadingEvent),
    Movement(AppMovementEvent),
    Combat(CombatRuntimeEvent),
    Menu(crate::game::MenuEvent),
    Explore(AppExploreEvent),
    Inventory(crate::game::InventoryEvent),
    Dialog(crate::game::DialogEvent),
    Shop(crate::game::ShopEvent),
    PauseMenu(crate::game::PauseMenuEvent),
    Transition(TransitionEvent),
    Exit(i32),
}

pub enum TransitionEvent {
    MapChanged,
    ToExplore,
    ToMenuFromGameOver,
    ReleaseMovementDirection(Direction),
}

pub enum AppExploreEvent {
    MoveDirection(Direction),
    Npc(crate::game::NpcEvent),
    UseAction(crate::game::ExploreAction),
    EnterPauseMenu,
    EnterMenu,
}

pub enum AppMovementEvent {
    Tick(
        crate::game::MovementTickEvent,
        Option<crate::game::TileEvent>,
    ),
}

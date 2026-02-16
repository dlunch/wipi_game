use crate::data::Direction;
use crate::game::state::CombatState;

pub enum RuntimeEvent {
    Loading(crate::game::LoadingEvent),
    Movement(AppMovementEvent),
    Menu(crate::game::MenuEvent),
    Explore(AppExploreEvent),
    Inventory(crate::game::InventoryEvent),
    Dialog(crate::game::DialogEvent),
    Shop(crate::game::ShopEvent),
    PauseMenu(crate::game::PauseMenuEvent),
    SetCombatState(CombatState),
    SetSkillCooldowns([u32; 3]),
    SetMpRegenTimer(u32),
    RecoverMp(i32),
    TakeDamage(i32),
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

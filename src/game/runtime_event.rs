use crate::data::Direction;
use crate::game::state::{FieldEnemy, SkillEffect};

pub enum CombatRuntimeEvent {
    SetEnemies(alloc::vec::Vec<FieldEnemy>),
    SetPlayerAttackCooldown(u32),
    SetPlayerHitFlash(u32),
    SetSkillEffects(alloc::vec::Vec<SkillEffect>),
    SetUpdateCounter(u32),
    SetRespawnTimer(u32),
    SetSkillCooldowns([u32; 3]),
    SetMpRegenTimer(u32),
    RecoverMp(i32),
    TakeDamage(i32),
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

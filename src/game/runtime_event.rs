use crate::data::Direction;
use crate::game::state::{FieldEnemy, SkillEffect};
use alloc::string::String;

#[derive(Clone)]
pub enum CombatRuntimeEvent {
    EnemySpawn(FieldEnemy),
    EnemyDespawn(u32),
    EnemyMove { enemy_id: u32, x: usize, y: usize },
    EnemyHpSet { enemy_id: u32, hp: i32 },
    EnemyAttackCooldownSet { enemy_id: u32, cooldown: u32 },
    EnemyHitFlashSet { enemy_id: u32, hit_flash: u32 },
    SetPlayerAttackCooldown(u32),
    SetPlayerHitFlash(u32),
    SetSkillEffects(alloc::vec::Vec<SkillEffect>),
    SetUpdateCounter(u32),
    SetRespawnTimer(u32),
    SetNextEnemyInstanceId(u32),
    SetSkillCooldowns([u32; 3]),
    SetMpRegenTimer(u32),
    RecoverMp(i32),
    TakeDamage(i32),
}

pub enum GameEvent {
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    StartNewGame,
    ContinueGame,
    OpenPauseMenu,
    OpenMenuFromExplore,
    OpenDialogState(crate::game::DialogState),
    OpenShopById(String),
    RestoreSessionStats,
    ApplyDialogAction(crate::data::DialogAction),
    ApplyDialogTransition(crate::game::DialogTransition),
    CombatPlayerAction(crate::game::ExploreAction),
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

pub enum UiEvent {
    OverlayCloseRequested,
    GameOverConfirmRequested,
    ErrorConfirmRequested,
    MovementKeyReleased(Direction),
    MenuInput(crate::game::InputKey),
    ExploreInput(crate::game::InputKey),
    InventoryInput(crate::game::InputKey),
    DialogInput(crate::game::InputKey),
    PauseMenuInput(crate::game::InputKey),
    ShopBuySelected(usize),
    ShopSellSelected(usize),
    ShopClose,
}

pub type RuntimeEvent = GameEvent;

#[derive(Clone, Copy)]
pub enum TransitionEvent {
    MapChanged,
    ToExplore,
    ToMenuFromGameOver,
    ReleaseMovementDirection(Direction),
}

#[derive(Clone)]
pub enum AppExploreEvent {
    MoveDirection(Direction),
    Npc(crate::game::NpcEvent),
    UseAction(crate::game::ExploreAction),
    EnterPauseMenu,
    EnterMenu,
}

#[derive(Clone)]
pub enum AppMovementEvent {
    Tick(
        crate::game::MovementTickEvent,
        Option<crate::game::TileEvent>,
    ),
}

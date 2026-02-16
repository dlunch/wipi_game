use crate::data::Direction;
use crate::game::state::{FieldEnemy, SkillEffect};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
pub enum CombatRuntimeEvent {
    EnemySpawn(FieldEnemy),
    EnemyDespawn(u32),
    EnemyMove {
        enemy_id: u32,
        x: usize,
        y: usize,
    },
    EnemyHpSet {
        enemy_id: u32,
        hp: i32,
    },
    EnemyAttackCooldownSet {
        enemy_id: u32,
        cooldown: u32,
    },
    EnemyHitFlashSet {
        enemy_id: u32,
        hit_flash: u32,
    },
    SetPlayerAttackCooldown(u32),
    SetPlayerHitFlash(u32),
    SetSkillEffects(Vec<SkillEffect>),
    SetUpdateCounter(u32),
    SetRespawnTimer(u32),
    SetNextEnemyInstanceId(u32),
    SetSkillCooldowns([u32; 3]),
    SetMpRegenTimer(u32),
    RecoverMp(i32),
    Heal(i32),
    GrantKillReward {
        enemy_id: String,
        exp: i32,
        gold: i32,
    },
    TakeDamage(i32),
}

pub enum GameEvent {
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    ExploreCommand(ExploreCommand),
    DialogCommand(crate::game::DialogCommand),
    ShopCommand(crate::game::ShopCommand),
    StartNewGame,
    ContinueGame,
    Session(SessionEvent),
    OpenPauseMenu,
    OpenMenuFromExplore,
    OpenDialogState(crate::game::DialogState),
    OpenShopById(String),
    OpenShopState(Box<crate::game::ShopState>),
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
    Lifecycle(crate::game::LifecycleEvent),
    Transition(TransitionEvent),
    Exit(i32),
}

#[derive(Clone)]
pub enum SessionEvent {
    Create,
    SetPlayerName(String),
    SetPlayerStats(crate::data::PlayerStats),
    SetPlayerMap(String),
    SetPlayerPosition { x: usize, y: usize },
    SetPlayerFacing(Direction),
    AddPlayerItem(crate::data::Item),
    SetEquippedWeapon(Option<usize>),
    SetEquippedArmor(Option<usize>),
    SetEquippedAccessory(Option<usize>),
    AddQuestProgress(crate::data::QuestProgress),
    AddOpenedTreasure { map_id: String, x: usize, y: usize },
    SetSkillCooldowns([u32; 3]),
    SetMpRegenTimer(u32),
    ResetMovement,
    ResetCombat,
    SpawnCurrentMapEnemies,
}

#[derive(Clone, Copy)]
pub enum ExploreCommand {
    Move(Direction),
    Confirm,
    UseSlot(usize),
    OpenPauseMenu,
    OpenMenu,
}

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
    TryNpcInteract {
        facing: Direction,
        fallback_action: Option<crate::game::ExploreAction>,
    },
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

use crate::data::Direction;
use crate::game::state::{FieldEnemy, SkillEffect};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
pub enum CombatEvent {
    SetUpdateCounter(u32),
    SetMapEnemies {
        enemies: Vec<FieldEnemy>,
        respawn_positions: Vec<(usize, usize, usize)>,
        next_enemy_instance_id: u32,
    },
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
    TickSkillEffects,
    SetSkillEffects(Vec<SkillEffect>),
    SetRespawnTimer(u32),
    SetNextEnemyInstanceId(u32),
    SetSkillCooldowns([u32; 3]),
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
    UpdateMovement,
    UpdateCombat,
    ExploreCommand(crate::game::ExploreCommand),
    StartNewGame,
    ContinueGame,
    SaveWorld,
    UseInventorySelected(usize),
    World(WorldEvent),
    OpenDialogState(crate::game::DialogState),
    OpenShopById(String),
    OpenShopState(Box<crate::game::ShopState>),
    RestoreHpMp,
    ApplyDialogAction(crate::data::DialogAction),
    ApplyDialogTransition(crate::game::DialogTransition),
    ShopBuyItem(crate::data::Item),
    ShopSellSelected(usize),
    CombatPlayerAction(crate::game::ExploreAction),
    Loading(crate::game::LoadingEvent),
    Movement(MovementEvent),
    Combat(CombatEvent),
    Explore(ExploreEvent),
    Lifecycle(crate::game::LifecycleEvent),
    Transition(TransitionEvent),
    Exit(i32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameEventKind {
    UpdateMovement,
    UpdateCombat,
    ExploreCommand,
    StartNewGame,
    ContinueGame,
    SaveWorld,
    UseInventorySelected,
    World,
    OpenDialogState,
    OpenShopById,
    OpenShopState,
    RestoreHpMp,
    ApplyDialogAction,
    ApplyDialogTransition,
    ShopBuyItem,
    ShopSellSelected,
    CombatPlayerAction,
    Loading,
    Movement,
    Combat,
    Explore,
    Lifecycle,
    Transition,
    Exit,
}

pub trait GameEventSubscriber {
    fn subscribes(&self, kind: GameEventKind) -> bool;
}

impl GameEventKind {
    pub const COUNT: usize = 24;

    pub const fn as_usize(self) -> usize {
        match self {
            Self::UpdateMovement => 0,
            Self::UpdateCombat => 1,
            Self::ExploreCommand => 2,
            Self::StartNewGame => 3,
            Self::ContinueGame => 4,
            Self::SaveWorld => 5,
            Self::UseInventorySelected => 6,
            Self::World => 7,
            Self::OpenDialogState => 8,
            Self::OpenShopById => 9,
            Self::OpenShopState => 10,
            Self::RestoreHpMp => 11,
            Self::ApplyDialogAction => 12,
            Self::ApplyDialogTransition => 13,
            Self::ShopBuyItem => 14,
            Self::ShopSellSelected => 15,
            Self::CombatPlayerAction => 16,
            Self::Loading => 17,
            Self::Movement => 18,
            Self::Combat => 19,
            Self::Explore => 20,
            Self::Lifecycle => 21,
            Self::Transition => 22,
            Self::Exit => 23,
        }
    }
}

impl GameEvent {
    pub const fn kind(&self) -> GameEventKind {
        match self {
            Self::UpdateMovement => GameEventKind::UpdateMovement,
            Self::UpdateCombat => GameEventKind::UpdateCombat,
            Self::ExploreCommand(_) => GameEventKind::ExploreCommand,
            Self::StartNewGame => GameEventKind::StartNewGame,
            Self::ContinueGame => GameEventKind::ContinueGame,
            Self::SaveWorld => GameEventKind::SaveWorld,
            Self::UseInventorySelected(_) => GameEventKind::UseInventorySelected,
            Self::World(_) => GameEventKind::World,
            Self::OpenDialogState(_) => GameEventKind::OpenDialogState,
            Self::OpenShopById(_) => GameEventKind::OpenShopById,
            Self::OpenShopState(_) => GameEventKind::OpenShopState,
            Self::RestoreHpMp => GameEventKind::RestoreHpMp,
            Self::ApplyDialogAction(_) => GameEventKind::ApplyDialogAction,
            Self::ApplyDialogTransition(_) => GameEventKind::ApplyDialogTransition,
            Self::ShopBuyItem(_) => GameEventKind::ShopBuyItem,
            Self::ShopSellSelected(_) => GameEventKind::ShopSellSelected,
            Self::CombatPlayerAction(_) => GameEventKind::CombatPlayerAction,
            Self::Loading(_) => GameEventKind::Loading,
            Self::Movement(_) => GameEventKind::Movement,
            Self::Combat(_) => GameEventKind::Combat,
            Self::Explore(_) => GameEventKind::Explore,
            Self::Lifecycle(_) => GameEventKind::Lifecycle,
            Self::Transition(_) => GameEventKind::Transition,
            Self::Exit(_) => GameEventKind::Exit,
        }
    }
}

#[derive(Clone)]
pub enum WorldEvent {
    Create,
    SetPlayerName(String),
    SetPlayerStats(crate::data::PlayerStats),
    SetPlayerInventory(Vec<crate::data::Item>),
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
}

#[derive(Clone, Copy)]
pub enum TransitionEvent {
    MapChanged,
    ToExplore,
    ToMenu,
    ToPauseMenu,
    ToInventory,
    ToStats,
    ToQuestLog,
    ToGameOver,
    ToMenuFromGameOver,
    ReleaseMovementDirection(Direction),
}

#[derive(Clone)]
pub enum ExploreEvent {
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
pub enum MovementEvent {
    Tick(
        crate::game::MovementTickEvent,
        Option<crate::game::TileEvent>,
    ),
}

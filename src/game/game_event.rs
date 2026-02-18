use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::data::Direction;
use crate::game::state::{
    CombatStatsSnapshot, EntityId, EntityStat, EntityState, ItemStack, LoadoutState, PartyState,
    TimedKind,
};

#[derive(Clone)]
#[allow(dead_code)]
pub enum CombatEvent {
    SetActive(bool),
    ClearAllies,
    ClearEnemies,
    SpawnEntity {
        entity_id: EntityId,
        kind: CombatSpawnKind,
    },
    RemoveEnemy(EntityId),
    MoveEnemy {
        entity_id: EntityId,
        x: usize,
        y: usize,
    },
    SetCombatantStats {
        entity_id: EntityId,
        stats: CombatStatsSnapshot,
    },
    SetCombatantTimed {
        entity_id: EntityId,
        kind: TimedKind,
        time_left: u32,
    },
    SetUpdateCounter(u32),
    SetRespawnTimer(u32),
    GrantKillReward {
        enemy_id: String,
        exp: i32,
        gold: i32,
    },
    RecoverMp {
        entity_id: EntityId,
        amount: i32,
    },
    Heal {
        entity_id: EntityId,
        amount: i32,
    },
    TakeDamage {
        entity_id: EntityId,
        amount: i32,
    },
}

#[derive(Clone)]
pub enum CombatSpawnKind {
    Ally,
    Enemy { source_enemy_id: String },
}

pub enum GameEvent {
    UpdateMovement,
    UpdateCombat,
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
    ShopBuyItem(String),
    ShopSellSelected(usize),
    RevivePlayer,
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
    RevivePlayer,
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
            Self::StartNewGame => 2,
            Self::ContinueGame => 3,
            Self::SaveWorld => 4,
            Self::UseInventorySelected => 5,
            Self::World => 6,
            Self::OpenDialogState => 7,
            Self::OpenShopById => 8,
            Self::OpenShopState => 9,
            Self::RestoreHpMp => 10,
            Self::ApplyDialogAction => 11,
            Self::ApplyDialogTransition => 12,
            Self::ShopBuyItem => 13,
            Self::ShopSellSelected => 14,
            Self::RevivePlayer => 15,
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
            Self::RevivePlayer => GameEventKind::RevivePlayer,
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
#[allow(dead_code)]
pub enum WorldEvent {
    CreateWorld,
    SetWorldMap(String),
    SetParty(PartyState),
    UpsertEntity(EntityState),
    RemoveEntity(EntityId),
    SetEntityMap {
        entity_id: EntityId,
        map_id: String,
    },
    SetEntityPosition {
        entity_id: EntityId,
        x: usize,
        y: usize,
    },
    SetEntityFacing {
        entity_id: EntityId,
        facing: Direction,
    },
    SetEntityStat {
        entity_id: EntityId,
        stat: EntityStat,
    },
    SetEntityInventory {
        entity_id: EntityId,
        inventory: Vec<ItemStack>,
    },
    SetEntityLoadout {
        entity_id: EntityId,
        loadout: LoadoutState,
    },
    AddEntityItem {
        entity_id: EntityId,
        item_id: String,
        amount: i32,
    },
    RemoveEntityItem {
        entity_id: EntityId,
        item_id: String,
        amount: i32,
    },
    AddQuestProgress(crate::data::QuestProgress),
    AddOpenedTreasure {
        map_id: String,
        x: usize,
        y: usize,
    },
    ResetMovement,
    ResetCombat,
}

#[derive(Clone, Copy)]
pub enum TransitionEvent {
    MapChanged,
    ToExplore,
    ToDead,
    ToMenu,
    ToPauseMenu,
    ToInventory,
    ToStats,
    ToQuestLog,
    ReleaseMovementDirection(Direction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileEvent {
    Treasure,
    MapExit(String),
    DungeonEntrance(String),
}

#[derive(Clone)]
pub enum ExploreEvent {
    MoveDirection(Direction),
    TryNpcInteract {
        facing: Direction,
        fallback_action: Option<crate::game::ExploreAction>,
    },
    Npc(crate::game::NpcEvent),
}

#[derive(Clone)]
pub enum MovementEvent {
    Tick(crate::game::MovementTickEvent, Option<TileEvent>),
}

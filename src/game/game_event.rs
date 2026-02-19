use alloc::{string::String, vec::Vec};

use crate::{
    data::{DialogAction, Direction},
    game::{
        state::{EntityId, EntityKind, MovementTickEvent, TimedKind},
        systems::{
            lifecycle::{LifecycleEvent, LoadingEvent},
            npc::NpcEvent,
        },
        ui::state::{DialogState, DialogTransition, ExploreAction},
    },
};

pub enum CombatEvent {
    SetActive(bool),
    ClearEnemies,
    RemoveEnemy(EntityId),
    MoveEnemy {
        entity_id: EntityId,
        x: usize,
        y: usize,
    },
    SetCombatantTimed {
        entity_id: EntityId,
        kind: TimedKind,
        end_tick: u32,
    },
    SetRespawnTimer(u32),
    GrantKillReward {
        enemy_id: String,
        exp: i32,
        gold: i32,
    },
}

pub enum GameEvent {
    Tick,
    SoftError(String),
    StartNewGame,
    ContinueGame,
    SaveWorld,
    UseInventorySelected(usize),
    World(WorldEvent),
    Entity(EntityEvent),
    OpenDialogState(DialogState),
    OpenShopById(String),
    SetShopBuyItemIds(Vec<u32>),
    SetShopSellItemIds(Vec<u32>),
    RestoreHpMp,
    ApplyDialogAction(DialogAction),
    ApplyDialogTransition(DialogTransition),
    ShopBuyItem(u32),
    ShopSellItem(u32),
    RevivePlayer,
    CombatPlayerAction(ExploreAction),
    FatalError(String),
    Loading(LoadingEvent),
    Movement(MovementEvent),
    Combat(CombatEvent),
    Explore(ExploreEvent),
    Lifecycle(LifecycleEvent),
    Transition(TransitionEvent),
    Exit(i32),
}

#[derive(PartialEq, Eq)]
pub enum GameEventKind {
    Tick,
    SoftError,
    StartNewGame,
    ContinueGame,
    SaveWorld,
    UseInventorySelected,
    World,
    Entity,
    OpenDialogState,
    OpenShopById,
    SetShopBuyItemIds,
    SetShopSellItemIds,
    RestoreHpMp,
    ApplyDialogAction,
    ApplyDialogTransition,
    ShopBuyItem,
    ShopSellItem,
    RevivePlayer,
    CombatPlayerAction,
    FatalError,
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
    pub const COUNT: usize = 27;

    pub const fn as_usize(&self) -> usize {
        match self {
            Self::Tick => 0,
            Self::SoftError => 1,
            Self::StartNewGame => 2,
            Self::ContinueGame => 3,
            Self::SaveWorld => 4,
            Self::UseInventorySelected => 5,
            Self::World => 6,
            Self::Entity => 7,
            Self::OpenDialogState => 8,
            Self::OpenShopById => 9,
            Self::SetShopBuyItemIds => 10,
            Self::SetShopSellItemIds => 11,
            Self::RestoreHpMp => 12,
            Self::ApplyDialogAction => 13,
            Self::ApplyDialogTransition => 14,
            Self::ShopBuyItem => 15,
            Self::ShopSellItem => 16,
            Self::RevivePlayer => 17,
            Self::CombatPlayerAction => 18,
            Self::FatalError => 19,
            Self::Loading => 20,
            Self::Movement => 21,
            Self::Combat => 22,
            Self::Explore => 23,
            Self::Lifecycle => 24,
            Self::Transition => 25,
            Self::Exit => 26,
        }
    }
}

impl GameEvent {
    pub const fn kind(&self) -> GameEventKind {
        match self {
            Self::Tick => GameEventKind::Tick,
            Self::SoftError(_) => GameEventKind::SoftError,
            Self::StartNewGame => GameEventKind::StartNewGame,
            Self::ContinueGame => GameEventKind::ContinueGame,
            Self::SaveWorld => GameEventKind::SaveWorld,
            Self::UseInventorySelected(_) => GameEventKind::UseInventorySelected,
            Self::World(_) => GameEventKind::World,
            Self::Entity(_) => GameEventKind::Entity,
            Self::OpenDialogState(_) => GameEventKind::OpenDialogState,
            Self::OpenShopById(_) => GameEventKind::OpenShopById,
            Self::SetShopBuyItemIds(_) => GameEventKind::SetShopBuyItemIds,
            Self::SetShopSellItemIds(_) => GameEventKind::SetShopSellItemIds,
            Self::RestoreHpMp => GameEventKind::RestoreHpMp,
            Self::ApplyDialogAction(_) => GameEventKind::ApplyDialogAction,
            Self::ApplyDialogTransition(_) => GameEventKind::ApplyDialogTransition,
            Self::ShopBuyItem(_) => GameEventKind::ShopBuyItem,
            Self::ShopSellItem(_) => GameEventKind::ShopSellItem,
            Self::RevivePlayer => GameEventKind::RevivePlayer,
            Self::CombatPlayerAction(_) => GameEventKind::CombatPlayerAction,
            Self::FatalError(_) => GameEventKind::FatalError,
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

pub enum WorldEvent {
    CreateWorld,
    SetWorldMap(String),
    CreateQuestProgress { quest_id: String },
    ChangeQuestCurrentCount { quest_id: String, delta: i32 },
    SetQuestCompleted { quest_id: String, completed: bool },
    SetQuestRewarded { quest_id: String, rewarded: bool },
    AddOpenedTreasure { map_id: String, x: usize, y: usize },
}

pub enum EntityEvent {
    SetLeaderEntity(EntityId),
    ClearCompanionEntities,
    AddCompanionEntity(EntityId),
    CreateEntity {
        entity_id: EntityId,
        kind: EntityKind,
        name: String,
    },
    SetEntityTransform {
        entity_id: EntityId,
        map_id: Option<String>,
        position: Option<(usize, usize)>,
        facing: Option<Direction>,
    },
    SetEntityLevel {
        entity_id: EntityId,
        level: i32,
    },
    SetEntityExp {
        entity_id: EntityId,
        exp: i32,
    },
    SetEntityExpToNext {
        entity_id: EntityId,
        exp_to_next: i32,
    },
    SetEntityBaseMaxHp {
        entity_id: EntityId,
        base_max_hp: i32,
    },
    SetEntityBaseMaxMp {
        entity_id: EntityId,
        base_max_mp: i32,
    },
    SetEntityBaseAtk {
        entity_id: EntityId,
        base_atk: i32,
    },
    SetEntityBaseDef {
        entity_id: EntityId,
        base_def: i32,
    },
    SetEntityCurrentHp {
        entity_id: EntityId,
        value: i32,
    },
    ChangeEntityHp {
        entity_id: EntityId,
        delta: i32,
    },
    SetEntityCurrentMp {
        entity_id: EntityId,
        value: i32,
    },
    ChangeEntityMp {
        entity_id: EntityId,
        delta: i32,
    },
    AddEntityExp {
        entity_id: EntityId,
        amount: i32,
    },
    ClearEntityInventory {
        entity_id: EntityId,
    },
    SetEntityLoadoutSlot {
        entity_id: EntityId,
        slot: LoadoutSlot,
        index: Option<usize>,
    },
    ChangeEntityItem {
        entity_id: EntityId,
        item_id: String,
        delta: i32,
    },
}

pub enum LoadoutSlot {
    Weapon,
    Armor,
    Accessory,
}

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

#[derive(Debug, PartialEq, Eq)]
pub enum TileEvent {
    Treasure,
    MapExit(String),
    DungeonEntrance(String),
}

pub enum ExploreEvent {
    MoveDirection(Direction),
    TryNpcInteract {
        facing: Direction,
        fallback_action: Option<ExploreAction>,
    },
    Npc(NpcEvent),
}

pub enum MovementEvent {
    Tick(MovementTickEvent, Option<TileEvent>),
    ClearPressedDirections,
    SetMoveCooldown(u32),
}

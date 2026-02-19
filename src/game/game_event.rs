use alloc::{string::String, vec::Vec};

use crate::{
    data::{DialogAction, Direction},
    game::{
        state::{EntityId, EntityKind, MovementTickEvent, TimedKind},
        systems::{
            lifecycle::{LifecycleEvent, LoadingEvent},
            npc::NpcEvent,
        },
        ui::state::ExploreAction,
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
        enemy_id: u32,
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
    OpenDialog { dialog_id: u32, npc_id: u32 },
    OpenShopById(u32),
    SetShopBuyItemIds(Vec<u32>),
    SetShopSellItemIds(Vec<u32>),
    RestoreHpMp,
    ApplyDialogAction(DialogAction),
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
    OpenDialog,
    OpenShopById,
    SetShopBuyItemIds,
    SetShopSellItemIds,
    RestoreHpMp,
    ApplyDialogAction,
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
    pub const COUNT: usize = 26;

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
            Self::OpenDialog => 8,
            Self::OpenShopById => 9,
            Self::SetShopBuyItemIds => 10,
            Self::SetShopSellItemIds => 11,
            Self::RestoreHpMp => 12,
            Self::ApplyDialogAction => 13,
            Self::ShopBuyItem => 14,
            Self::ShopSellItem => 15,
            Self::RevivePlayer => 16,
            Self::CombatPlayerAction => 17,
            Self::FatalError => 18,
            Self::Loading => 19,
            Self::Movement => 20,
            Self::Combat => 21,
            Self::Explore => 22,
            Self::Lifecycle => 23,
            Self::Transition => 24,
            Self::Exit => 25,
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
            Self::OpenDialog { .. } => GameEventKind::OpenDialog,
            Self::OpenShopById(_) => GameEventKind::OpenShopById,
            Self::SetShopBuyItemIds(_) => GameEventKind::SetShopBuyItemIds,
            Self::SetShopSellItemIds(_) => GameEventKind::SetShopSellItemIds,
            Self::RestoreHpMp => GameEventKind::RestoreHpMp,
            Self::ApplyDialogAction(_) => GameEventKind::ApplyDialogAction,
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
    SetWorldMap(u32),
    CreateQuestProgress { quest_id: u32 },
    ChangeQuestCurrentCount { quest_id: u32, delta: i32 },
    SetQuestCompleted { quest_id: u32, completed: bool },
    SetQuestRewarded { quest_id: u32, rewarded: bool },
    AddOpenedTreasure { map_id: u32, x: usize, y: usize },
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
        map_id: Option<u32>,
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
        item_id: u32,
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
    MapExit(u32),
    DungeonEntrance(u32),
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

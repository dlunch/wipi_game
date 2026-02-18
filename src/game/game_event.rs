use alloc::boxed::Box;
use alloc::string::String;

use crate::data::Direction;
use crate::game::state::{EntityId, EntityKind, TimedKind};

pub enum CombatEvent {
    SetActive(bool),
    ClearEnemies,
    RemoveEnemy(EntityId),
    MoveEnemy {
        entity_id: EntityId,
        x: usize,
        y: usize,
    },
    SetCombatantMaxHp {
        entity_id: EntityId,
        max_hp: i32,
    },
    SetCombatantCurrentHp {
        entity_id: EntityId,
        current_hp: i32,
    },
    SetCombatantMaxMp {
        entity_id: EntityId,
        max_mp: i32,
    },
    SetCombatantCurrentMp {
        entity_id: EntityId,
        current_mp: i32,
    },
    SetCombatantAtk {
        entity_id: EntityId,
        atk: i32,
    },
    SetCombatantDef {
        entity_id: EntityId,
        def: i32,
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

pub enum GameEvent {
    UpdateMovement,
    UpdateCombat,
    StartNewGame,
    ContinueGame,
    SaveWorld,
    UseInventorySelected(usize),
    World(WorldEvent),
    Entity(EntityEvent),
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
    Entity,
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
    pub const COUNT: usize = 25;

    pub const fn as_usize(self) -> usize {
        match self {
            Self::UpdateMovement => 0,
            Self::UpdateCombat => 1,
            Self::StartNewGame => 2,
            Self::ContinueGame => 3,
            Self::SaveWorld => 4,
            Self::UseInventorySelected => 5,
            Self::World => 6,
            Self::Entity => 7,
            Self::OpenDialogState => 8,
            Self::OpenShopById => 9,
            Self::OpenShopState => 10,
            Self::RestoreHpMp => 11,
            Self::ApplyDialogAction => 12,
            Self::ApplyDialogTransition => 13,
            Self::ShopBuyItem => 14,
            Self::ShopSellSelected => 15,
            Self::RevivePlayer => 16,
            Self::CombatPlayerAction => 17,
            Self::Loading => 18,
            Self::Movement => 19,
            Self::Combat => 20,
            Self::Explore => 21,
            Self::Lifecycle => 22,
            Self::Transition => 23,
            Self::Exit => 24,
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
            Self::Entity(_) => GameEventKind::Entity,
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

#[derive(Clone, Copy)]
pub enum LoadoutSlot {
    Weapon,
    Armor,
    Accessory,
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
        fallback_action: Option<crate::game::ExploreAction>,
    },
    Npc(crate::game::NpcEvent),
}

pub enum MovementEvent {
    Tick(crate::game::MovementTickEvent, Option<TileEvent>),
    ClearPressedDirections,
    SetMoveCooldown(u32),
}

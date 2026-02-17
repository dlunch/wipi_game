mod effects;
mod game_data;
mod game_event;
mod rendering;
mod save;
mod save_schema;
mod selection;
mod state;
mod systems;
mod ui;
mod world;

pub use effects::apply_effects;
pub use game_data::GameData;
pub use game_event::{
    CombatEvent, ExploreEvent, GameEvent, GameEventKind, GameEventSubscriber, MovementEvent,
    TileEvent, TransitionEvent, WorldEvent,
};
pub use rendering::{
    ExploreRender, InventoryRender, QuestLogRender, RenderFxState, RenderState, ShopRender,
    SpriteAtlas, StatsRender, render,
};
pub(crate) use state::WorldSlot;
#[allow(unused_imports)]
pub use state::{
    AllyCombatantState, CombatState, CombatStatsSnapshot, CombatantState, EnemyCombatantState,
    EntityId, EntityKind, EntityStat, EntityState, EntityStore, GOLD_ITEM_ID, GameState, ItemStack,
    LoadoutState, MovementState, MovementTickEvent, PartyState, TimedEffect, TimedKind, TimedState,
};
pub use systems::{DomainEventResolver, LifecycleEvent, LoadingEvent, NpcEvent, domain_resolvers};
pub use ui::{
    DialogState, DialogTransition, ExploreAction, GameInput, InputKey, MenuAction, ShopMode,
    ShopState, UiEvent, UiEventApplier, UiInputEventResolver, UiState,
};
pub use world::WorldState;

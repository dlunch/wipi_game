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

pub use effects::{DomainEventEffect, domain_effects};
pub use game_data::GameData;
pub use game_event::{
    CombatEvent, EntityEvent, ExploreEvent, GameEvent, GameEventKind, GameEventSubscriber,
    LoadoutSlot, MovementEvent, TileEvent, TransitionEvent, WorldEvent,
};
pub use rendering::{
    ExploreRender, InventoryRender, QuestLogRender, RenderFxState, RenderState, ShopRender,
    SpriteAtlas, StatsRender, render,
};
pub(crate) use state::WorldSlot;
pub use state::{
    EntityStat, EntityState, EntityStore, GOLD_ITEM_ID, GameState, MovementState, MovementTickEvent,
};
pub use systems::{DomainEventResolver, LifecycleEvent, LoadingEvent, NpcEvent, domain_resolvers};
pub use ui::{
    DialogState, DialogTransition, ExploreAction, GameInput, InputKey, MenuAction, ShopMode,
    ShopState, UiEventApplier, UiInputEventResolver, UiState,
};
pub use world::WorldState;

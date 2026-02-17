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
    TransitionEvent, WorldEvent,
};
pub use rendering::{
    ExploreRender, InventoryRender, QuestLogRender, RenderFxState, RenderState, ShopRender,
    StatsRender, render,
};
pub(crate) use state::WorldSlot;
pub use state::{
    CharacterState, CombatState, GameState, MovementState, MovementTickEvent, TileEvent,
};
pub use systems::{
    DomainEventResolver, LifecycleEvent, LoadingEvent, NpcEvent, ResolveContext, domain_resolvers,
};
pub use ui::{
    DialogState, DialogTransition, ExploreAction, GameInput, InputKey, MenuAction, ShopMode,
    ShopState, UiEvent, UiEventApplier, UiInputEventResolver, UiState,
};
pub use world::WorldState;

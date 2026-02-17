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

pub use game_data::GameData;
pub use game_event::{
    CombatEvent, ExploreEvent, GameEvent, GameEventKind, GameEventSubscriber, MovementEvent,
    TransitionEvent, WorldEvent,
};
pub use rendering::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, ExploreRender,
    InventoryRender, QuestLogRender, RenderFxState, RenderState, ShopRender, StatsRender,
    apply_render_event, apply_render_tick, apply_ui_render_patch, clear_screen, draw_dialog,
    draw_explore, draw_inventory, draw_menu, draw_pause_menu, draw_quest_log, draw_rect, draw_shop,
    draw_stats, draw_text, fill_rect, render,
};
pub use save::{has_save_data, load_game, save_game};
pub use state::{
    CharacterState, CombatState, GameState, MovementState, MovementTickEvent, TileEvent,
};
pub use systems::{
    DomainEventResolver, LifecycleEvent, LoadingEvent, NpcEvent, ResolveContext, domain_resolvers,
};
pub use ui::{
    DialogState, DialogTransition, ExploreAction, ExploreCommand, GameInput,
    INVENTORY_VISIBLE_ITEMS, InputKey, MenuAction, MenuState, SHOP_VISIBLE_ITEMS, ShopMode,
    ShopState, UiEvent, UiEventApplier, UiInputEventResolver, UiState,
};
pub use world::WorldState;

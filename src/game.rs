mod game_data;
mod intent;
mod rendering;
mod runtime_event;
mod save;
mod save_schema;
mod selection;
mod session;
mod state;
mod systems;
mod ui;

pub use game_data::GameData;
pub use intent::{GameInput, InputKey};
pub use rendering::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, ExploreRender,
    InventoryRender, QuestLogRender, RenderState, ShopRender, StatsRender, build_render_state,
    clear_screen, draw_dialog, draw_explore, draw_inventory, draw_menu, draw_pause_menu,
    draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect, render,
};
pub use runtime_event::{
    AppExploreEvent, AppMovementEvent, CombatRuntimeEvent, RuntimeEvent, TransitionEvent,
};
pub use save::{has_save_data, load_game, save_game};
pub use session::{DialogActionResult, SessionEventApplier, SessionState};
pub use state::{
    CombatAction, CombatEvent, CombatState, GameState, MovementState, MovementTickEvent,
    PlayerAction, PlayerEffect, PlayerEvent, PlayerState, TileApplyEvent, TileEvent,
    domain_appliers,
};
pub use systems::npc;
pub use systems::{
    ApplyContext, DialogEvent, DialogIntent, DialogTransition, ExploreIntent, InventoryEvent,
    InventoryIntent, LoadingEvent, MenuEvent, MenuIntent, NpcEvent, PauseMenuEvent,
    PauseMenuIntent, ResolveContext, ShopEvent, ShopIntent, domain_resolvers,
};
pub use ui::{
    DialogState, ExploreAction, INVENTORY_VISIBLE_ITEMS, MenuAction, MenuState, SHOP_VISIBLE_ITEMS,
    ShopMode, ShopState, UiInputEventResolver, UiState,
};

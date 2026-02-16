mod game_data;
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
pub use rendering::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, ExploreRender,
    InventoryRender, QuestLogRender, RenderState, ShopRender, StatsRender, build_render_state,
    clear_screen, draw_dialog, draw_explore, draw_inventory, draw_menu, draw_pause_menu,
    draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect, render,
};
pub use runtime_event::{
    AppExploreEvent, AppMovementEvent, CombatRuntimeEvent, GameEvent, ShopInputEvent,
    TransitionEvent, UiEvent,
};
pub use save::{has_save_data, load_game, save_game};
pub use session::{SessionState, continue_game, enter_session, start_new_game};
pub use state::{
    CombatState, GameState, MovementState, MovementTickEvent, PlayerAction, PlayerEvent,
    PlayerState, TileApplyEvent, TileEvent,
};
pub use systems::{
    DialogEvent, DialogTransition, InventoryEvent, LoadingEvent, MenuEvent, NpcEvent,
    PauseMenuEvent, ResolveContext, ShopEvent, domain_resolvers,
};
pub use ui::{
    DialogState, ExploreAction, GameInput, INVENTORY_VISIBLE_ITEMS, InputKey, MenuAction,
    MenuState, SHOP_VISIBLE_ITEMS, ShopMode, ShopState, UiEventApplier, UiInputEventResolver,
    UiState,
};

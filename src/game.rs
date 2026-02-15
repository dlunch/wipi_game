mod game_data;
mod intent;
mod rendering;
mod save;
mod session;
mod state;
mod systems;
mod ui;

pub use game_data::GameData;
pub use intent::{AppAction, AppEffect};
pub use rendering::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, ExploreRender,
    InventoryRender, QuestLogRender, RenderState, ShopRender, StatsRender, build_render_state,
    clear_screen, draw_dialog, draw_explore, draw_inventory, draw_menu, draw_pause_menu,
    draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect, render,
};
pub use save::{has_save_data, load_game, save_game};
pub use session::SessionState;
pub use state::{GameState, PlayerState};
pub use systems::combat;
pub use systems::dialog;
pub use systems::explore;
pub use systems::inventory;
pub use systems::lifecycle;
pub use systems::menu;
pub use systems::movement;
pub use systems::npc;
pub use systems::player;
pub use systems::quest;
pub use systems::reward;
pub use systems::shop;
pub use systems::{
    CombatEvent, CombatIntent, CombatState, DialogEvent, DialogIntent, DialogTransition,
    ExploreEvent, ExploreIntent, InventoryEvent, InventoryIntent, MenuEvent, MenuIntent,
    MovementState, NpcEvent, NpcIntent, PauseMenuEvent, PauseMenuIntent, PlayerEvent, PlayerIntent,
    QuestIntent, ShopEvent, ShopIntent,
};
pub use ui::{
    DialogState, ExploreAction, ExploreUiState, INVENTORY_VISIBLE_ITEMS, InventoryUiState,
    MenuAction, MenuState, MenuUiState, PauseMenuUiState, SHOP_VISIBLE_ITEMS, ShopMode,
    ShopState, ShopUiState, UiState,
};

pub mod app;
mod game_data;
mod rendering;
mod save;
mod state;
mod systems;

pub use app::RpgGame;
pub use game_data::GameData;
pub use rendering::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, clear_screen, draw_dialog,
    draw_explore, draw_inventory, draw_loading, draw_menu, draw_pause_menu, draw_quest_log,
    draw_rect, draw_shop, draw_stats, draw_text, fill_rect, render,
};
pub use save::{has_save_data, load_game, save_game};
pub use state::{
    DialogIntent, DialogState, GameState, InventoryIntent, InventoryState, MenuAction, MenuIntent,
    MenuState, PauseMenuIntent, PlayerState, ShopIntent, ShopMode, ShopState, TileEvent,
    check_tile_event, pause_menu_intent_for_key,
};
pub use systems::NpcIntent;
pub use systems::combat;
pub use systems::movement;
pub use systems::npc;
pub use systems::player;
pub use systems::quest;
pub use systems::{CombatEvent, CombatIntent, CombatState, Direction, PlayerEffect};
pub use systems::{MovementState, PlayerEvent, PlayerIntent};

use wipi::framebuffer::Framebuffer;

use crate::data::{Dialog, Map, Npc, Quest};
use crate::game::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, CombatState, DialogState,
    GameData, GameState, InventoryState, MenuState, PauseMenuState, PlayerState, SessionState,
    ShopState, clear_screen, draw_dialog, draw_explore, draw_inventory, draw_menu, draw_pause_menu,
    draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect,
};

pub struct ExploreRenderState<'a> {
    pub map: &'a Map,
    pub player: &'a PlayerState,
    pub combat: &'a CombatState,
    pub npcs: &'a [Npc],
    pub skill_cooldowns: &'a [u32; 3],
}

pub enum RenderState<'a> {
    Loading {
        step: usize,
    },
    Menu(&'a MenuState),
    Explore(ExploreRenderState<'a>),
    Inventory {
        player: &'a PlayerState,
        state: &'a InventoryState,
    },
    Stats(&'a PlayerState),
    Dialog {
        explore: Option<ExploreRenderState<'a>>,
        dialog_state: &'a DialogState,
        dialogs: &'a [Dialog],
    },
    Shop {
        shop_state: &'a ShopState,
        player: &'a PlayerState,
    },
    QuestLog {
        player: &'a PlayerState,
        quests: &'a [Quest],
    },
    PauseMenu {
        explore: Option<ExploreRenderState<'a>>,
        state: &'a PauseMenuState,
    },
    GameOver,
    Error(&'a str),
    NoSession,
}

fn map_for_session<'a>(session: &'a SessionState, data: &'a GameData) -> Option<&'a Map> {
    data.find_map(&session.player.current_map_id)
}

fn build_explore_state<'a>(
    session: &'a SessionState,
    data: &'a GameData,
) -> Option<ExploreRenderState<'a>> {
    let map = map_for_session(session, data)?;
    Some(ExploreRenderState {
        map,
        player: &session.player,
        combat: &session.combat,
        npcs: &data.npcs,
        skill_cooldowns: &session.skill_cooldowns,
    })
}

pub fn build_render_state<'a>(
    state: &'a GameState,
    session: Option<&'a SessionState>,
    data: &'a GameData,
) -> RenderState<'a> {
    match state {
        GameState::Loading(step) => RenderState::Loading { step: *step },
        GameState::Menu(menu_state) => RenderState::Menu(menu_state),
        GameState::Explore => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            let Some(explore) = build_explore_state(s, data) else {
                return RenderState::Error("Map not found");
            };
            RenderState::Explore(explore)
        }
        GameState::Inventory => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            RenderState::Inventory {
                player: &s.player,
                state: &s.inventory,
            }
        }
        GameState::Stats => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            RenderState::Stats(&s.player)
        }
        GameState::Dialog(dialog_state) => RenderState::Dialog {
            explore: session.and_then(|s| build_explore_state(s, data)),
            dialog_state,
            dialogs: &data.dialogs,
        },
        GameState::Shop(shop_state) => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            RenderState::Shop {
                shop_state,
                player: &s.player,
            }
        }
        GameState::QuestLog => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            RenderState::QuestLog {
                player: &s.player,
                quests: &data.quests,
            }
        }
        GameState::PauseMenu(state) => RenderState::PauseMenu {
            explore: session.and_then(|s| build_explore_state(s, data)),
            state,
        },
        GameState::GameOver => RenderState::GameOver,
        GameState::Error(msg) => RenderState::Error(msg.as_str()),
    }
}

pub fn render(state: &RenderState<'_>, fb: &mut Framebuffer) {
    match state {
        RenderState::Loading { step } => draw_loading(fb, *step),
        RenderState::Menu(menu_state) => draw_menu(fb, menu_state),
        RenderState::Explore(explore) => {
            draw_explore(
                fb,
                explore.map,
                explore.player,
                explore.combat,
                explore.npcs,
                explore.skill_cooldowns,
            );
        }
        RenderState::Inventory { player, state } => draw_inventory(fb, player, state),
        RenderState::Stats(player) => draw_stats(fb, player),
        RenderState::Dialog {
            explore,
            dialog_state,
            dialogs,
        } => {
            if let Some(explore) = explore {
                draw_explore(
                    fb,
                    explore.map,
                    explore.player,
                    explore.combat,
                    explore.npcs,
                    explore.skill_cooldowns,
                );
            }
            draw_dialog(fb, dialog_state, dialogs);
        }
        RenderState::Shop { shop_state, player } => draw_shop(fb, shop_state, player),
        RenderState::QuestLog { player, quests } => draw_quest_log(fb, player, quests),
        RenderState::PauseMenu { explore, state } => {
            if let Some(explore) = explore {
                draw_explore(
                    fb,
                    explore.map,
                    explore.player,
                    explore.combat,
                    explore.npcs,
                    explore.skill_cooldowns,
                );
            }
            draw_pause_menu(fb, state);
        }
        RenderState::GameOver => {
            clear_screen(fb);
            let w = fb.width() as i32;
            let h = fb.height() as i32;
            fill_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_DARK_GRAY);
            draw_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_RED);
            draw_text(fb, w / 2 - 35, h / 2 - 8, "GAME OVER", COLOR_RED);
            draw_text(fb, w / 2 - 30, h / 2 + 8, "OK:Menu", COLOR_WHITE);
        }
        RenderState::Error(msg) => {
            clear_screen(fb);
            let w = fb.width() as i32;
            let h = fb.height() as i32;
            fill_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_DARK_GRAY);
            draw_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_RED);
            draw_text(fb, 16, h / 2 - 20, "ERROR", COLOR_RED);
            draw_text(fb, 16, h / 2 - 4, msg, COLOR_WHITE);
            draw_text(fb, 16, h / 2 + 16, "OK:Exit", COLOR_WHITE);
        }
        RenderState::NoSession => {
            clear_screen(fb);
            draw_text(fb, 16, 16, "ERR: No session", COLOR_RED);
        }
    }
}

fn draw_loading(fb: &mut Framebuffer, step: usize) {
    clear_screen(fb);

    let w = fb.width() as i32;
    let h = fb.height() as i32;

    draw_text(fb, w / 2 - 30, h / 2 - 30, "Loading...", COLOR_WHITE);

    let clamped = step.min(GameData::LOAD_STEPS - 1);
    let label = GameData::LOAD_LABELS[clamped];
    draw_text(fb, w / 2 - 30, h / 2 - 10, label, COLOR_CYAN);

    let bar_w = 120;
    let bar_h = 8;
    let bar_x = w / 2 - bar_w / 2;
    let bar_y = h / 2 + 10;

    draw_rect(fb, bar_x, bar_y, bar_w, bar_h, COLOR_WHITE);
    let progress = (((clamped + 1) * bar_w as usize / GameData::LOAD_STEPS) as i32).min(bar_w);
    fill_rect(
        fb,
        bar_x + 1,
        bar_y + 1,
        progress - 2,
        bar_h - 2,
        COLOR_GREEN,
    );
}

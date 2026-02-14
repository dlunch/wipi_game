use wipi::framebuffer::Framebuffer;

use crate::game::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, GameData, GameState,
    SessionState, clear_screen, draw_dialog, draw_explore, draw_inventory, draw_menu,
    draw_pause_menu, draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect,
};

fn require_session<'a>(
    session: Option<&'a SessionState>,
    fb: &mut Framebuffer,
) -> Option<&'a SessionState> {
    if session.is_some() {
        return session;
    }
    clear_screen(fb);
    draw_text(fb, 16, 16, "ERR: No session", COLOR_RED);
    None
}

pub fn render(
    state: &GameState,
    session: Option<&SessionState>,
    data: &GameData,
    fb: &mut Framebuffer,
) {
    match state {
        GameState::Loading(step) => draw_loading(fb, *step),
        GameState::Menu(menu_state) => draw_menu(fb, menu_state),
        GameState::Explore => {
            let Some(s) = require_session(session, fb) else {
                return;
            };
            if let Some(map) = data.find_map(&s.player.current_map_id) {
                draw_explore(fb, map, &s.player, &s.combat, &data.npcs, s);
            }
        }
        GameState::Inventory => {
            let Some(s) = require_session(session, fb) else {
                return;
            };
            draw_inventory(fb, &s.player, &s.inventory);
        }
        GameState::Stats => {
            let Some(s) = require_session(session, fb) else {
                return;
            };
            draw_stats(fb, &s.player);
        }
        GameState::Dialog(dialog_state) => {
            if let Some(s) = session
                && let Some(map) = data.find_map(&s.player.current_map_id)
            {
                draw_explore(fb, map, &s.player, &s.combat, &data.npcs, s);
            }
            draw_dialog(fb, dialog_state, data);
        }
        GameState::Shop(shop_state) => {
            let Some(s) = require_session(session, fb) else {
                return;
            };
            draw_shop(fb, shop_state, &s.player);
        }
        GameState::QuestLog => {
            let Some(s) = require_session(session, fb) else {
                return;
            };
            draw_quest_log(fb, &s.player, &data.quests);
        }
        GameState::PauseMenu(selected) => {
            if let Some(s) = session
                && let Some(map) = data.find_map(&s.player.current_map_id)
            {
                draw_explore(fb, map, &s.player, &s.combat, &data.npcs, s);
            }
            draw_pause_menu(fb, *selected);
        }
        GameState::GameOver => {
            clear_screen(fb);
            let w = fb.width() as i32;
            let h = fb.height() as i32;
            fill_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_DARK_GRAY);
            draw_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_RED);
            draw_text(fb, w / 2 - 35, h / 2 - 8, "GAME OVER", COLOR_RED);
            draw_text(fb, w / 2 - 30, h / 2 + 8, "OK:Menu", COLOR_WHITE);
        }
        GameState::Error(msg) => {
            clear_screen(fb);
            let w = fb.width() as i32;
            let h = fb.height() as i32;
            fill_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_DARK_GRAY);
            draw_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_RED);
            draw_text(fb, 16, h / 2 - 20, "ERROR", COLOR_RED);
            draw_text(fb, 16, h / 2 - 4, msg, COLOR_WHITE);
            draw_text(fb, 16, h / 2 + 16, "OK:Exit", COLOR_WHITE);
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

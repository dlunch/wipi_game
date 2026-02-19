use alloc::format;

use wipi::framebuffer::Framebuffer;

use super::{
    dialog::draw_dialog,
    explore::draw_explore,
    inventory::{draw_inventory, draw_stats},
    menu::{draw_menu, draw_pause_menu},
    quest::draw_quest_log,
    render_fx::RenderFxState,
    render_state::RenderState,
    renderer::{
        COLOR_BLUE, COLOR_DARK_GRAY, COLOR_RED, COLOR_WHITE, clear_screen, draw_rect, draw_text,
        fill_rect,
    },
    shop::draw_shop,
    sprites::SpriteAtlas,
};

pub fn render(
    state: &RenderState,
    sprites: &SpriteAtlas,
    render_fx: &RenderFxState,
    fb: &mut Framebuffer,
) {
    match state {
        RenderState::Loading { step } => draw_loading(fb, *step),
        RenderState::Menu {
            title,
            items,
            selected,
        } => draw_menu(fb, title, items, *selected),
        RenderState::Explore(explore) => draw_explore(fb, explore, sprites),
        RenderState::Inventory(inventory) => draw_inventory(fb, inventory),
        RenderState::Stats(stats) => draw_stats(fb, stats),
        RenderState::Dialog {
            explore,
            npc_name,
            current_text,
            has_next,
            ..
        } => {
            if let Some(explore_state) = explore {
                draw_explore(fb, explore_state, sprites);
            }
            draw_dialog(fb, npc_name, current_text.as_deref(), *has_next);
        }
        RenderState::Shop(shop) => draw_shop(fb, shop),
        RenderState::QuestLog(quest_log) => draw_quest_log(fb, quest_log),
        RenderState::PauseMenu {
            explore,
            items,
            selected,
        } => {
            if let Some(explore_state) = explore {
                draw_explore(fb, explore_state, sprites);
            }
            draw_pause_menu(fb, items, *selected);
        }
        RenderState::Dead => draw_dead(fb),
        RenderState::Error(msg) => draw_error(fb, msg),
    }

    if render_fx.soft_error_notice_timer() > 0
        && let Some(message) = render_fx.soft_error_message()
    {
        draw_soft_error_notice(fb, message);
    }
}

fn draw_loading(fb: &mut Framebuffer, step: usize) {
    const TOTAL_LOADING_STEPS: usize = 8;

    clear_screen(fb);
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let clamped_step = step.min(TOTAL_LOADING_STEPS);
    let percent = (clamped_step * 100) / TOTAL_LOADING_STEPS;

    draw_text(fb, w / 2 - 30, h / 2 - 10, "LOADING", COLOR_WHITE);
    let bar_w = (w - 40).max(80);
    let bar_h = 12;
    let bar_x = (w - bar_w) / 2;
    let bar_y = h / 2 + 12;
    fill_rect(fb, bar_x, bar_y, bar_w, bar_h, COLOR_DARK_GRAY);
    let fill_w = ((bar_w - 2) as usize * clamped_step / TOTAL_LOADING_STEPS) as i32;
    if fill_w > 0 {
        fill_rect(fb, bar_x + 1, bar_y + 1, fill_w, bar_h - 2, COLOR_BLUE);
    }
    draw_rect(fb, bar_x, bar_y, bar_w, bar_h, COLOR_WHITE);

    draw_text(
        fb,
        w / 2 - 28,
        h / 2 + 30,
        &format!("{}% ({}/{})", percent, clamped_step, TOTAL_LOADING_STEPS),
        COLOR_WHITE,
    );
}

fn draw_soft_error_notice(fb: &mut Framebuffer, message: &str) {
    let width = fb.width() as i32;
    let box_x = 12;
    let box_y = 8;
    let box_w = width - 24;
    let box_h = 24;
    fill_rect(fb, box_x, box_y, box_w, box_h, COLOR_DARK_GRAY);
    draw_rect(fb, box_x, box_y, box_w, box_h, COLOR_RED);
    draw_text(fb, box_x + 8, box_y + 8, message, COLOR_WHITE);
}

fn draw_dead(fb: &mut Framebuffer) {
    clear_screen(fb);
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    fill_rect(fb, w / 2 - 52, h / 2 - 24, 104, 48, COLOR_DARK_GRAY);
    draw_rect(fb, w / 2 - 52, h / 2 - 24, 104, 48, COLOR_RED);
    draw_text(fb, w / 2 - 35, h / 2 - 10, "YOU DIED", COLOR_RED);
    draw_text(fb, w / 2 - 43, h / 2 + 8, "OK: Revive", COLOR_WHITE);
}

fn draw_error(fb: &mut Framebuffer, msg: &str) {
    clear_screen(fb);
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    fill_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_DARK_GRAY);
    draw_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_RED);
    draw_text(fb, 16, h / 2 - 20, "ERROR", COLOR_RED);
    draw_text(fb, 16, h / 2 - 4, msg, COLOR_WHITE);
    draw_text(fb, 16, h / 2 + 16, "OK:Exit", COLOR_WHITE);
}

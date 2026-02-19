use wipi::framebuffer::Framebuffer;

use super::renderer::{
    COLOR_DARK_GRAY, COLOR_GRAY, COLOR_RED, COLOR_WHITE, COLOR_YELLOW, clear_screen, draw_rect,
    draw_selection_cursor, draw_text, fill_rect,
};
use crate::game::ui::state::MenuAction;

pub fn draw_menu(
    fb: &mut Framebuffer,
    title: &'static str,
    items: &[(&'static str, MenuAction)],
    selected: usize,
) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;

    fill_rect(fb, 20, 10, screen_w - 40, 24, COLOR_DARK_GRAY);
    draw_rect(fb, 20, 10, screen_w - 40, 24, COLOR_WHITE);
    draw_text(fb, 35, 18, title, COLOR_YELLOW);

    let menu_y_start: i32 = 50;
    let menu_spacing: i32 = 18;

    for (i, (label, _)) in items.iter().enumerate() {
        let y = menu_y_start + (i as i32) * menu_spacing;

        if i == selected {
            draw_selection_cursor(fb, 28, y);
            fill_rect(fb, 36, y, 70, 12, COLOR_DARK_GRAY);
        }

        draw_rect(
            fb,
            36,
            y,
            70,
            12,
            if i == selected {
                COLOR_WHITE
            } else {
                COLOR_GRAY
            },
        );
        draw_text(
            fb,
            40,
            y + 2,
            label,
            if i == selected {
                COLOR_WHITE
            } else {
                COLOR_GRAY
            },
        );
    }

    draw_text(fb, 20, 120, "OK:Select", COLOR_GRAY);
}

pub fn draw_pause_menu(fb: &mut Framebuffer, items: &[&'static str], selected: usize) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let menu_w = 100;
    let menu_h = 80;
    let x = (w - menu_w) / 2;
    let y = (h - menu_h) / 2;

    fill_rect(fb, x, y, menu_w, menu_h, COLOR_DARK_GRAY);
    draw_rect(fb, x, y, menu_w, menu_h, COLOR_WHITE);

    for (i, item) in items.iter().enumerate() {
        let is_selected = i == selected;
        let prefix = if is_selected { "> " } else { "  " };
        let y_pos = y + 10 + (i as i32 * 16);
        let color = if is_selected { COLOR_RED } else { COLOR_WHITE };
        draw_text(fb, x + 10, y_pos, prefix, color);
        draw_text(fb, x + 22, y_pos, item, color);
    }
}

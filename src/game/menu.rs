use wipi::framebuffer::Framebuffer;

use super::MenuState;
use super::renderer::{
    COLOR_DARK_GRAY, COLOR_GRAY, COLOR_WHITE, COLOR_YELLOW, clear_screen, draw_rect,
    draw_selection_cursor, draw_text, fill_rect,
};

pub fn draw_menu(fb: &mut Framebuffer, state: &MenuState) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;

    fill_rect(fb, 20, 10, screen_w - 40, 24, COLOR_DARK_GRAY);
    draw_rect(fb, 20, 10, screen_w - 40, 24, COLOR_WHITE);
    draw_text(fb, 35, 18, "LOST KINGDOM", COLOR_YELLOW);

    let menu_y_start: i32 = 50;
    let menu_spacing: i32 = 18;

    let items: &[&str] = if state.has_save {
        &["NEW GAME", "CONTINUE", "EXIT"]
    } else {
        &["NEW GAME", "EXIT"]
    };

    for (i, item) in items.iter().enumerate() {
        let y = menu_y_start + (i as i32) * menu_spacing;

        if i == state.selected {
            draw_selection_cursor(fb, 28, y);
            fill_rect(fb, 36, y, 70, 12, COLOR_DARK_GRAY);
        }

        draw_rect(
            fb,
            36,
            y,
            70,
            12,
            if i == state.selected {
                COLOR_WHITE
            } else {
                COLOR_GRAY
            },
        );
        draw_text(
            fb,
            40,
            y + 2,
            item,
            if i == state.selected {
                COLOR_WHITE
            } else {
                COLOR_GRAY
            },
        );
    }

    draw_text(fb, 20, 120, "OK:Select", COLOR_GRAY);
}

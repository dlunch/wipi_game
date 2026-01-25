use wipi::framebuffer::Framebuffer;

use super::renderer::{
    COLOR_BLACK, COLOR_GRAY, COLOR_WHITE, COLOR_YELLOW, clear_screen, draw_rect, draw_text,
    fill_rect,
};
use super::state::DialogState;

pub fn draw_dialog(fb: &mut Framebuffer, state: &DialogState) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    let box_h = 40;
    let box_y = screen_h - box_h - 4;

    fill_rect(fb, 4, box_y, screen_w - 8, box_h, COLOR_BLACK);
    draw_rect(fb, 4, box_y, screen_w - 8, box_h, COLOR_WHITE);

    draw_text(fb, 8, box_y + 2, &state.npc_name, COLOR_YELLOW);

    if let Some(text) = state.current_text() {
        let max_chars = ((screen_w - 16) / 6) as usize;
        let lines = wrap_text(text, max_chars);

        for (i, line) in lines.iter().take(2).enumerate() {
            draw_text(fb, 8, box_y + 12 + (i as i32 * 10), line, COLOR_WHITE);
        }
    }

    let indicator = if state.current_line + 1 < state.lines.len() {
        "OK:Next"
    } else {
        "OK:Close"
    };
    draw_text(fb, screen_w - 50, box_y + box_h - 10, indicator, COLOR_GRAY);
}

fn wrap_text(text: &str, max_chars: usize) -> alloc::vec::Vec<&str> {
    let mut lines = alloc::vec::Vec::new();
    let mut start = 0;
    let chars: alloc::vec::Vec<_> = text.char_indices().collect();

    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let end_byte = if end < chars.len() {
            chars[end].0
        } else {
            text.len()
        };
        let start_byte = chars[start].0;
        lines.push(&text[start_byte..end_byte]);
        start = end;
    }

    if lines.is_empty() {
        lines.push("");
    }

    lines
}

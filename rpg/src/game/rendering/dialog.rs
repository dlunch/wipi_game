use alloc::vec::Vec;

use wipi::framebuffer::Framebuffer;

use super::renderer::{
    COLOR_BLACK, COLOR_GRAY, COLOR_WHITE, COLOR_YELLOW, draw_rect, draw_text, fill_rect,
};

pub fn draw_dialog(
    fb: &mut Framebuffer,
    npc_name: &str,
    current_text: Option<&str>,
    has_next: bool,
) {
    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    let box_h = 40;
    let box_y = screen_h - box_h - 4;

    fill_rect(fb, 4, box_y, screen_w - 8, box_h, COLOR_BLACK);
    draw_rect(fb, 4, box_y, screen_w - 8, box_h, COLOR_WHITE);

    draw_text(fb, 8, box_y + 2, npc_name, COLOR_YELLOW);

    if let Some(text) = current_text {
        let max_chars = ((screen_w - 16) / 6) as usize;
        let lines = wrap_text(text, max_chars);

        for (i, line) in lines.iter().take(2).enumerate() {
            draw_text(fb, 8, box_y + 12 + (i as i32 * 10), line, COLOR_WHITE);
        }
    }

    let indicator = if has_next { "OK:Next" } else { "OK:Close" };
    draw_text(fb, screen_w - 50, box_y + box_h - 10, indicator, COLOR_GRAY);
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<&str> {
    let mut lines = Vec::new();

    if max_chars == 0 || text.is_empty() {
        lines.push("");
        return lines;
    }

    let mut start = 0;
    let chars = text.char_indices().collect::<Vec<_>>();

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

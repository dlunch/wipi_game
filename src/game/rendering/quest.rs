use alloc::format;

use wipi::framebuffer::Framebuffer;

use super::renderer::{
    COLOR_BLACK, COLOR_GRAY, COLOR_GREEN, COLOR_WHITE, COLOR_YELLOW, clear_screen, draw_rect,
    draw_text, fill_rect,
};
use crate::game::QuestLogRender;

pub fn draw_quest_log(fb: &mut Framebuffer, state: &QuestLogRender) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_BLACK);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 6, "Quest Log", COLOR_YELLOW);

    if state.quests.is_empty() {
        draw_text(fb, 8, 24, "No active quests", COLOR_GRAY);
    } else {
        for (i, quest) in state.quests.iter().enumerate() {
            let y = 24 + (i as i32 * 24);
            if y > screen_h - 30 {
                break;
            }

            let color = if quest.completed {
                COLOR_GREEN
            } else {
                COLOR_WHITE
            };
            draw_text(fb, 8, y, &quest.name, color);

            let progress_text = format!("{}/{}", quest.current_count, quest.target_count);
            draw_text(fb, screen_w - 40, y, &progress_text, color);

            let desc_y = y + 10;
            let max_chars = ((screen_w - 16) / 6) as usize;
            let desc = truncate_by_chars(&quest.description, max_chars);
            draw_text(fb, 12, desc_y, desc, COLOR_GRAY);
        }
    }

    draw_text(fb, 8, screen_h - 14, "Back:Close", COLOR_GRAY);
}

fn truncate_by_chars(s: &str, max_chars: usize) -> &str {
    if max_chars == 0 {
        return "";
    }
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}

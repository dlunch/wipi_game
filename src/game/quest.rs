use alloc::format;
use alloc::vec::Vec;
use wipi::framebuffer::Framebuffer;

use super::Player;
use super::renderer::{
    COLOR_BLACK, COLOR_GRAY, COLOR_GREEN, COLOR_WHITE, COLOR_YELLOW, clear_screen, draw_rect,
    draw_text, fill_rect,
};
use crate::data::Quest;

pub fn draw_quest_log(fb: &mut Framebuffer, player: &Player, quests: &[Quest]) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_BLACK);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 6, "Quest Log", COLOR_YELLOW);

    let active_quests: Vec<_> = player
        .quests
        .iter()
        .filter(|p| !p.rewarded)
        .filter_map(|p| quests.iter().find(|q| q.id == p.quest_id).map(|q| (p, q)))
        .collect();

    if active_quests.is_empty() {
        draw_text(fb, 8, 24, "No active quests", COLOR_GRAY);
    } else {
        for (i, (progress, quest)) in active_quests.iter().enumerate() {
            let y = 24 + (i as i32 * 24);
            if y > screen_h - 30 {
                break;
            }

            let (status_color1, status_color2) = if progress.completed {
                (COLOR_GREEN, COLOR_GREEN)
            } else {
                (COLOR_WHITE, COLOR_WHITE)
            };
            draw_text(fb, 8, y, &quest.name, status_color1);

            let progress_text = format!("{}/{}", progress.current_count, quest.target_count);
            draw_text(fb, screen_w - 40, y, &progress_text, status_color2);

            let desc_y = y + 10;
            let max_len = ((screen_w - 16) / 6) as usize;
            let desc = if quest.description.len() > max_len {
                &quest.description[..max_len]
            } else {
                &quest.description
            };
            draw_text(fb, 12, desc_y, desc, COLOR_GRAY);
        }
    }

    draw_text(fb, 8, screen_h - 14, "Back:Close", COLOR_GRAY);
}

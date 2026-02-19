use alloc::format;

use wipi::framebuffer::Framebuffer;

use super::{
    render_state::{InventoryRender, StatsRender},
    renderer::{
        COLOR_BLACK, COLOR_BLUE, COLOR_DARK_GRAY, COLOR_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE,
        COLOR_YELLOW, clear_screen, draw_hp_bar, draw_rect, draw_selection_cursor, draw_text,
        fill_rect,
    },
};
use crate::data::ItemKind;

pub fn draw_inventory(fb: &mut Framebuffer, state: &InventoryRender) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_DARK_GRAY);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 8, "INVENTORY", COLOR_YELLOW);

    if state.items.is_empty() {
        draw_text(fb, 30, 60, "No items", COLOR_GRAY);
        draw_text(fb, 8, screen_h - 16, "Back:Return", COLOR_GRAY);
        return;
    }

    let start_y = 24;
    let item_height = 14;
    let visible_items = ((screen_h - 50) / item_height).max(1) as usize;
    let scroll = state.scroll;

    for (i, item) in state
        .items
        .iter()
        .skip(scroll)
        .take(visible_items)
        .enumerate()
    {
        let actual_idx = scroll + i;
        let y = start_y + (i as i32) * item_height;

        let is_equipped = state.equipped_weapon == Some(actual_idx)
            || state.equipped_armor == Some(actual_idx)
            || state.equipped_accessory == Some(actual_idx);

        if actual_idx == state.selected {
            draw_selection_cursor(fb, 8, y);
        }

        let bg_color = if is_equipped { COLOR_BLUE } else { COLOR_BLACK };
        fill_rect(fb, 16, y, screen_w - 28, 12, bg_color);
        draw_rect(
            fb,
            16,
            y,
            screen_w - 28,
            12,
            if actual_idx == state.selected {
                COLOR_WHITE
            } else {
                COLOR_GRAY
            },
        );

        let type_indicator = match item.kind {
            ItemKind::Weapon => COLOR_RED,
            ItemKind::Armor => COLOR_BLUE,
            ItemKind::Accessory => COLOR_YELLOW,
            ItemKind::Consumable => COLOR_GREEN,
        };
        fill_rect(fb, 18, y + 2, 4, 8, type_indicator);

        let equip_mark = if is_equipped { "E " } else { "  " };
        let item_text = format!("{}{}", equip_mark, item.name);
        draw_text(fb, 24, y + 2, &item_text, COLOR_WHITE);
    }

    if state.items.len() > visible_items {
        if scroll > 0 {
            draw_text(fb, screen_w - 16, 24, "^", COLOR_WHITE);
        }
        if scroll + visible_items < state.items.len() {
            draw_text(
                fb,
                screen_w - 16,
                start_y + (visible_items as i32) * item_height - 8,
                "v",
                COLOR_WHITE,
            );
        }
    }

    draw_text(fb, 8, screen_h - 16, "OK:Use Back:Return", COLOR_GRAY);
}

pub fn draw_stats(fb: &mut Framebuffer, state: &StatsRender) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_DARK_GRAY);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 8, "STATUS", COLOR_YELLOW);

    let stat_y = 24;
    let line_height = 14;

    draw_text(fb, 10, stat_y, "HP", COLOR_WHITE);
    draw_hp_bar(fb, 30, stat_y + 2, 60, state.hp as i32, state.max_hp as i32);
    let hp_text = format!("{}/{}", state.hp, state.max_hp);
    draw_text(fb, 94, stat_y, &hp_text, COLOR_WHITE);

    draw_text(fb, 10, stat_y + line_height, "MP", COLOR_BLUE);
    let mp_fill = ((state.mp * 60) / state.max_mp) as i32;
    fill_rect(fb, 30, stat_y + line_height + 2, 60, 4, COLOR_DARK_GRAY);
    fill_rect(fb, 30, stat_y + line_height + 2, mp_fill, 4, COLOR_BLUE);
    draw_rect(fb, 30, stat_y + line_height + 2, 60, 4, COLOR_WHITE);
    let mp_text = format!("{}/{}", state.mp, state.max_mp);
    draw_text(fb, 94, stat_y + line_height, &mp_text, COLOR_WHITE);

    let stats = [
        ("LV", state.level),
        ("ATK", state.atk),
        ("DEF", state.def),
        ("EXP", state.exp),
        ("GOLD", state.gold),
    ];

    for (i, (label, value)) in stats.iter().enumerate() {
        let y = stat_y + ((i + 2) as i32) * line_height;
        let text = format!("{}: {}", label, value);
        draw_text(fb, 10, y, &text, COLOR_WHITE);
    }

    draw_text(fb, 8, screen_h - 16, "Back:Return", COLOR_GRAY);
}

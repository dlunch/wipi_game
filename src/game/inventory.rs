use alloc::format;
use wipi::framebuffer::Framebuffer;

use super::Player;
use super::renderer::{
    COLOR_BLACK, COLOR_BLUE, COLOR_DARK_GRAY, COLOR_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE,
    COLOR_YELLOW, clear_screen, draw_hp_bar, draw_rect, draw_selection_cursor, draw_text,
    fill_rect,
};
use crate::data::ItemKind;

#[derive(Default)]
pub struct InventoryState {
    pub selected: usize,
    pub scroll: usize,
}

impl InventoryState {
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll {
                self.scroll = self.selected;
            }
        }
    }

    pub fn move_down(&mut self, item_count: usize) {
        if item_count > 0 && self.selected < item_count - 1 {
            self.selected += 1;
            if self.selected >= self.scroll + 6 {
                self.scroll = self.selected - 5;
            }
        }
    }
}

pub fn draw_inventory(fb: &mut Framebuffer, player: &Player, state: &InventoryState) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_DARK_GRAY);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 8, "INVENTORY", COLOR_YELLOW);

    if player.inventory.is_empty() {
        draw_text(fb, 30, 60, "No items", COLOR_GRAY);
        draw_text(fb, 8, screen_h - 16, "Back:Return", COLOR_GRAY);
        return;
    }

    let visible_items: i32 = 6;
    let start_y: i32 = 24;
    let item_height: i32 = 14;

    for (i, item) in player
        .inventory
        .iter()
        .skip(state.scroll)
        .take(visible_items as usize)
        .enumerate()
    {
        let actual_idx = state.scroll + i;
        let y = start_y + (i as i32) * item_height;

        let is_equipped = player.equipped_weapon == Some(actual_idx)
            || player.equipped_armor == Some(actual_idx)
            || player.equipped_accessory == Some(actual_idx);

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

    if player.inventory.len() > visible_items as usize {
        if state.scroll > 0 {
            draw_text(fb, screen_w - 16, 24, "^", COLOR_WHITE);
        }
        if state.scroll + (visible_items as usize) < player.inventory.len() {
            draw_text(
                fb,
                screen_w - 16,
                start_y + visible_items * item_height - 8,
                "v",
                COLOR_WHITE,
            );
        }
    }

    draw_text(fb, 8, screen_h - 16, "OK:Use Back:Return", COLOR_GRAY);
}

pub fn draw_stats(fb: &mut Framebuffer, player: &Player) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_DARK_GRAY);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 8, "STATUS", COLOR_YELLOW);

    let stat_y: i32 = 24;
    let line_height: i32 = 14;

    draw_text(fb, 10, stat_y, "HP", COLOR_WHITE);
    draw_hp_bar(
        fb,
        30,
        stat_y + 2,
        60,
        player.stats.current_hp,
        player.stats.max_hp,
    );
    let hp_text = format!("{}/{}", player.stats.current_hp, player.stats.max_hp);
    draw_text(fb, 94, stat_y, &hp_text, COLOR_WHITE);

    draw_text(fb, 10, stat_y + line_height, "MP", COLOR_BLUE);
    let mp_fill = if player.stats.max_mp > 0 {
        (player.stats.current_mp * 60) / player.stats.max_mp
    } else {
        0
    };
    fill_rect(fb, 30, stat_y + line_height + 2, 60, 4, COLOR_DARK_GRAY);
    fill_rect(fb, 30, stat_y + line_height + 2, mp_fill, 4, COLOR_BLUE);
    draw_rect(fb, 30, stat_y + line_height + 2, 60, 4, COLOR_WHITE);
    let mp_text = format!("{}/{}", player.stats.current_mp, player.stats.max_mp);
    draw_text(fb, 94, stat_y + line_height, &mp_text, COLOR_WHITE);

    let stats = [
        ("LV", player.stats.level),
        ("ATK", player.total_atk()),
        ("DEF", player.total_def()),
        ("EXP", player.stats.exp),
        ("GOLD", player.stats.gold),
    ];

    for (i, (label, value)) in stats.iter().enumerate() {
        let y = stat_y + ((i + 2) as i32) * line_height;
        let text = format!("{}: {}", label, value);
        draw_text(fb, 10, y, &text, COLOR_WHITE);
    }

    draw_text(fb, 8, screen_h - 16, "Back:Return", COLOR_GRAY);
}

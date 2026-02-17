use alloc::format;

use wipi::framebuffer::Framebuffer;

use super::renderer::{
    COLOR_DARK_GRAY, COLOR_GRAY, COLOR_GREEN, COLOR_WHITE, COLOR_YELLOW, clear_screen, draw_rect,
    draw_text, fill_rect,
};
use crate::game::{ShopMode, ShopRender};

pub fn draw_shop(fb: &mut Framebuffer, state: &ShopRender) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_DARK_GRAY);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 6, &state.shop_name, COLOR_YELLOW);

    let gold_text = format!("Gold: {}", state.player_gold);
    draw_text(fb, screen_w - 60, 6, &gold_text, COLOR_YELLOW);

    match state.mode {
        ShopMode::Select => draw_mode_select(fb, state.selected),
        ShopMode::Buy => draw_buy_list(fb, state),
        ShopMode::ConfirmBuy => {
            draw_buy_list(fb, state);
            draw_buy_confirm(fb, state);
        }
        ShopMode::Sell => draw_sell_list(fb, state),
        ShopMode::ConfirmSell => {
            draw_sell_list(fb, state);
            draw_sell_confirm(fb, state);
        }
    }

    if state.purchase_notice_timer > 0 {
        draw_purchase_notice(fb);
    }

    draw_text(fb, 8, screen_h - 14, "Back:Exit", COLOR_GRAY);
}

fn draw_mode_select(fb: &mut Framebuffer, selected: usize) {
    let screen_w = fb.width() as i32;
    let center_x = screen_w / 2 - 30;

    let buy_color = if selected == 0 {
        COLOR_WHITE
    } else {
        COLOR_GRAY
    };
    let sell_color = if selected == 1 {
        COLOR_WHITE
    } else {
        COLOR_GRAY
    };

    if selected == 0 {
        draw_text(fb, center_x - 8, 30, ">", COLOR_YELLOW);
    }
    draw_text(fb, center_x, 30, "Buy", buy_color);

    if selected == 1 {
        draw_text(fb, center_x - 8, 42, ">", COLOR_YELLOW);
    }
    draw_text(fb, center_x, 42, "Sell", sell_color);
}

fn draw_buy_list(fb: &mut Framebuffer, state: &ShopRender) {
    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;
    let visible_items = ((screen_h - 60) / 12).max(1) as usize;

    draw_text(fb, 8, 18, "== BUY ==", COLOR_GREEN);

    for (i, item) in state
        .buy_items
        .iter()
        .skip(state.scroll)
        .take(visible_items)
        .enumerate()
    {
        let actual_idx = state.scroll + i;
        let y = 30 + (i as i32 * 12);

        let is_selected = actual_idx == state.selected;
        let can_afford = state.player_gold >= item.price;

        let (text_color1, text_color2) = if is_selected {
            (COLOR_WHITE, COLOR_WHITE)
        } else if can_afford {
            (COLOR_GRAY, COLOR_GRAY)
        } else {
            (COLOR_GRAY, COLOR_DARK_GRAY)
        };

        if is_selected {
            draw_text(fb, 8, y, ">", COLOR_YELLOW);
        }

        draw_text(fb, 16, y, &item.name, text_color1);

        let price_text = format!("{}G", item.price);
        draw_text(fb, screen_w - 40, y, &price_text, text_color2);
    }

    if state.scroll > 0 {
        draw_text(fb, screen_w - 16, 30, "^", COLOR_WHITE);
    }
    if state.scroll + visible_items < state.buy_items.len() {
        draw_text(
            fb,
            screen_w - 16,
            30 + ((visible_items - 1) as i32) * 12,
            "v",
            COLOR_WHITE,
        );
    }
}

fn draw_sell_list(fb: &mut Framebuffer, state: &ShopRender) {
    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;
    let visible_items = ((screen_h - 60) / 12).max(1) as usize;

    draw_text(fb, 8, 18, "== SELL ==", COLOR_GREEN);

    if state.player_inventory.is_empty() {
        draw_text(fb, 8, 30, "No items", COLOR_GRAY);
        return;
    }

    for (i, item) in state
        .player_inventory
        .iter()
        .skip(state.scroll)
        .take(visible_items)
        .enumerate()
    {
        let actual_idx = state.scroll + i;
        let y = 30 + (i as i32 * 12);

        let is_selected = actual_idx == state.selected;
        let (text_color1, text_color2) = if is_selected {
            (COLOR_WHITE, COLOR_WHITE)
        } else {
            (COLOR_GRAY, COLOR_GRAY)
        };

        if is_selected {
            draw_text(fb, 8, y, ">", COLOR_YELLOW);
        }

        draw_text(fb, 16, y, &item.name, text_color1);

        let price_text = format!("{}G", item.price);
        draw_text(fb, screen_w - 40, y, &price_text, text_color2);
    }

    if state.scroll > 0 {
        draw_text(fb, screen_w - 16, 30, "^", COLOR_WHITE);
    }
    if state.scroll + visible_items < state.player_inventory.len() {
        draw_text(
            fb,
            screen_w - 16,
            30 + ((visible_items - 1) as i32) * 12,
            "v",
            COLOR_WHITE,
        );
    }
}

fn draw_buy_confirm(fb: &mut Framebuffer, state: &ShopRender) {
    let Some(item) = state.buy_items.get(state.selected) else {
        return;
    };

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;
    let box_w = 150;
    let box_h = 34;
    let x = (screen_w - box_w) / 2;
    let y = screen_h - box_h - 24;

    fill_rect(fb, x, y, box_w, box_h, COLOR_DARK_GRAY);
    draw_rect(fb, x, y, box_w, box_h, COLOR_WHITE);

    draw_text(fb, x + 8, y + 8, &item.name, COLOR_WHITE);
    draw_text(fb, x + 8, y + 18, "Buy this item?", COLOR_YELLOW);
    draw_text(fb, x + box_w - 58, y + 18, "OK/Back", COLOR_GRAY);
}

fn draw_purchase_notice(fb: &mut Framebuffer) {
    let screen_w = fb.width() as i32;
    let box_w = 120;
    let box_h = 18;
    let x = (screen_w - box_w) / 2;
    let y = 8;

    fill_rect(fb, x, y, box_w, box_h, COLOR_DARK_GRAY);
    draw_rect(fb, x, y, box_w, box_h, COLOR_WHITE);
    draw_text(fb, x + 10, y + 5, "Purchased!", COLOR_GREEN);
}

fn draw_sell_confirm(fb: &mut Framebuffer, state: &ShopRender) {
    let Some(item) = state.player_inventory.get(state.selected) else {
        return;
    };

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;
    let box_w = 150;
    let box_h = 34;
    let x = (screen_w - box_w) / 2;
    let y = screen_h - box_h - 24;

    fill_rect(fb, x, y, box_w, box_h, COLOR_DARK_GRAY);
    draw_rect(fb, x, y, box_w, box_h, COLOR_WHITE);

    draw_text(fb, x + 8, y + 8, &item.name, COLOR_WHITE);
    draw_text(fb, x + 8, y + 18, "Sell this item?", COLOR_YELLOW);
    draw_text(fb, x + box_w - 58, y + 18, "OK/Back", COLOR_GRAY);
}

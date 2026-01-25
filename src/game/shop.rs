use alloc::format;
use wipi::framebuffer::Framebuffer;

use super::Player;
use super::renderer::{
    COLOR_BLACK, COLOR_BLUE, COLOR_DARK_GRAY, COLOR_GRAY, COLOR_GREEN, COLOR_WHITE, COLOR_YELLOW,
    clear_screen, draw_rect, draw_text, fill_rect,
};
use super::state::{ShopMode, ShopState};

pub fn draw_shop(fb: &mut Framebuffer, state: &ShopState, player: &Player) {
    clear_screen(fb);

    let screen_w = fb.width() as i32;
    let screen_h = fb.height() as i32;

    fill_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_BLACK);
    draw_rect(fb, 4, 4, screen_w - 8, screen_h - 8, COLOR_WHITE);

    draw_text(fb, 8, 6, &state.shop.name, COLOR_YELLOW);

    let gold_text = format!("Gold: {}", player.stats.gold);
    draw_text(fb, screen_w - 60, 6, &gold_text, COLOR_YELLOW);

    match state.mode {
        ShopMode::Select => draw_mode_select(fb, state),
        ShopMode::Buy => draw_buy_list(fb, state, player),
        ShopMode::Sell => draw_sell_list(fb, state, player),
    }

    draw_text(fb, 8, screen_h - 14, "Back:Exit", COLOR_GRAY);
}

fn draw_mode_select(fb: &mut Framebuffer, state: &ShopState) {
    let screen_w = fb.width() as i32;
    let center_x = screen_w / 2 - 30;

    let buy_color = if state.selected == 0 {
        COLOR_WHITE
    } else {
        COLOR_GRAY
    };
    let sell_color = if state.selected == 1 {
        COLOR_WHITE
    } else {
        COLOR_GRAY
    };

    if state.selected == 0 {
        draw_text(fb, center_x - 8, 30, ">", COLOR_YELLOW);
    }
    draw_text(fb, center_x, 30, "Buy", buy_color);

    if state.selected == 1 {
        draw_text(fb, center_x - 8, 42, ">", COLOR_YELLOW);
    }
    draw_text(fb, center_x, 42, "Sell", sell_color);
}

fn draw_buy_list(fb: &mut Framebuffer, state: &ShopState, player: &Player) {
    let screen_w = fb.width() as i32;

    draw_text(fb, 8, 18, "== BUY ==", COLOR_GREEN);

    for (i, item) in state.items.iter().enumerate() {
        let y = 30 + (i as i32 * 12);
        if y > 100 {
            break;
        }

        let is_selected = i == state.selected;
        let can_afford = player.stats.gold >= item.price;

        let (text_color1, text_color2) = if is_selected {
            (COLOR_WHITE, COLOR_WHITE)
        } else if can_afford {
            (COLOR_GRAY, COLOR_GRAY)
        } else {
            (COLOR_DARK_GRAY, COLOR_DARK_GRAY)
        };

        if is_selected {
            draw_text(fb, 8, y, ">", COLOR_YELLOW);
        }

        draw_text(fb, 16, y, &item.name, text_color1);

        let price_text = format!("{}G", item.price);
        draw_text(fb, screen_w - 40, y, &price_text, text_color2);
    }
}

fn draw_sell_list(fb: &mut Framebuffer, state: &ShopState, player: &Player) {
    let screen_w = fb.width() as i32;

    draw_text(fb, 8, 18, "== SELL ==", COLOR_BLUE);

    if player.inventory.is_empty() {
        draw_text(fb, 8, 30, "No items", COLOR_GRAY);
        return;
    }

    for (i, item) in player.inventory.iter().enumerate() {
        let y = 30 + (i as i32 * 12);
        if y > 100 {
            break;
        }

        let is_selected = i == state.selected;
        let (text_color1, text_color2) = if is_selected {
            (COLOR_WHITE, COLOR_WHITE)
        } else {
            (COLOR_GRAY, COLOR_GRAY)
        };

        if is_selected {
            draw_text(fb, 8, y, ">", COLOR_YELLOW);
        }

        draw_text(fb, 16, y, &item.name, text_color1);

        let sell_price = item.price / 2;
        let price_text = format!("{}G", sell_price);
        draw_text(fb, screen_w - 40, y, &price_text, text_color2);
    }
}

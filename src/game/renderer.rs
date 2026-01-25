use wipi::framebuffer::{Color, Framebuffer};

pub const TILE_SIZE: i32 = 8;

pub const COLOR_BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
pub const COLOR_WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
pub const COLOR_GRAY: Color = Color {
    r: 128,
    g: 128,
    b: 128,
    a: 255,
};
pub const COLOR_DARK_GRAY: Color = Color {
    r: 64,
    g: 64,
    b: 64,
    a: 255,
};
pub const COLOR_GREEN: Color = Color {
    r: 0,
    g: 200,
    b: 0,
    a: 255,
};
pub const COLOR_BLUE: Color = Color {
    r: 0,
    g: 100,
    b: 200,
    a: 255,
};
pub const COLOR_RED: Color = Color {
    r: 200,
    g: 50,
    b: 50,
    a: 255,
};
pub const COLOR_BROWN: Color = Color {
    r: 139,
    g: 90,
    b: 43,
    a: 255,
};
pub const COLOR_YELLOW: Color = Color {
    r: 255,
    g: 255,
    b: 0,
    a: 255,
};
pub const COLOR_CYAN: Color = Color {
    r: 0,
    g: 255,
    b: 255,
    a: 255,
};
pub const COLOR_DUNGEON: Color = Color {
    r: 150,
    g: 50,
    b: 50,
    a: 255,
};
pub const COLOR_FOREST: Color = Color {
    r: 34,
    g: 139,
    b: 34,
    a: 255,
};

pub fn clear_screen(fb: &mut Framebuffer) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    fb.fill_rect(0, 0, w, h, COLOR_BLACK);
}

pub fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, c: Color) {
    fb.fill_rect(x, y, w, h, c);
}

pub fn draw_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, c: Color) {
    fb.draw_rect(x, y, w, h, c);
}

pub fn draw_text(fb: &mut Framebuffer, x: i32, y: i32, text: &str, c: Color) {
    fb.draw_text(x, y, text, c);
}

pub fn draw_hp_bar(fb: &mut Framebuffer, x: i32, y: i32, w: i32, current: i32, max: i32) {
    fb.fill_rect(x, y, w, 4, COLOR_DARK_GRAY);

    let fill = if max > 0 { (current * w) / max } else { 0 };
    let c = if current * 4 < max {
        COLOR_RED
    } else if current * 2 < max {
        COLOR_YELLOW
    } else {
        COLOR_GREEN
    };
    fb.fill_rect(x, y, fill, 4, c);
    fb.draw_rect(x, y, w, 4, COLOR_WHITE);
}

pub fn draw_selection_cursor(fb: &mut Framebuffer, x: i32, y: i32) {
    fb.fill_rect(x, y + 2, 4, 4, COLOR_WHITE);
}

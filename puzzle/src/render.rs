use alloc::format;

use wipi::{
    framebuffer::{Color, Framebuffer},
    image::Image,
};

use crate::{
    game::Game,
    level::{MAX_HEIGHT, Tile},
};

const TILE_SIZE: i32 = 24;
const SCREEN_WIDTH: i32 = 240;
const SCREEN_HEIGHT: i32 = 320;
const BOARD_TOP: i32 = 28;

const BACKGROUND: Color = Color {
    r: 0x47,
    g: 0x6B,
    b: 0x82,
    a: 255,
};
const TEXT: Color = Color {
    r: 0xF4,
    g: 0xFA,
    b: 0xFC,
    a: 255,
};
// Text baselines.
const HUD_TOP_Y: i32 = 18;
const HUD_BOTTOM_Y: i32 = 310;

// Column order of the 24x24 cells in tiles.png.
const SPRITE_FLOOR: i32 = 0;
const SPRITE_WALL: i32 = 1;
const SPRITE_GOAL: i32 = 2;
const SPRITE_BOX: i32 = 3;
const SPRITE_BOX_ON_GOAL: i32 = 4;
const SPRITE_PLAYER: i32 = 5;
const SPRITE_PLAYER_ON_GOAL: i32 = 6;

pub fn render(game: &Game, tiles: &Image, fb: &mut Framebuffer) {
    fb.fill_rect(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, BACKGROUND);

    let level = &game.levels[game.current];
    let width = level.width as i32;
    let height = (level.tiles.len() / level.width) as i32;
    let left = (SCREEN_WIDTH - width * TILE_SIZE) / 2;
    let top = BOARD_TOP + (MAX_HEIGHT as i32 - height) * TILE_SIZE / 2;

    for (index, &tile) in level.tiles.iter().enumerate() {
        let on_goal = tile == Tile::Goal;
        let sprite = if tile == Tile::Wall {
            SPRITE_WALL
        } else if index == game.player {
            if on_goal {
                SPRITE_PLAYER_ON_GOAL
            } else {
                SPRITE_PLAYER
            }
        } else if game.boxes.contains(&index) {
            if on_goal {
                SPRITE_BOX_ON_GOAL
            } else {
                SPRITE_BOX
            }
        } else if on_goal {
            SPRITE_GOAL
        } else {
            SPRITE_FLOOR
        };
        fb.draw_image(
            left + index as i32 % width * TILE_SIZE,
            top + index as i32 / width * TILE_SIZE,
            TILE_SIZE as u32,
            TILE_SIZE as u32,
            tiles,
            sprite * TILE_SIZE,
            0,
        );
    }

    let stage = format!("STAGE {}/{}", game.current + 1, game.levels.len());
    fb.draw_text(8, HUD_TOP_Y, &stage, TEXT);
    fb.draw_text(
        144,
        HUD_TOP_Y,
        &format!("MOVES {}", game.history.len()),
        TEXT,
    );
    let hint = if game.is_solved() {
        "CLEAR!  OK:NEXT STAGE"
    } else {
        "1:UNDO 3:RESET 7/9:STAGE"
    };
    fb.draw_text(8, HUD_BOTTOM_Y, hint, TEXT);
}

pub fn render_error(error: &anyhow::Error, fb: &mut Framebuffer) {
    fb.fill_rect(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, BACKGROUND);
    fb.draw_text(8, HUD_TOP_Y, &format!("{error}"), TEXT);
}

use alloc::format;
use wipi::framebuffer::{Color, Framebuffer};

use super::Player;
use super::combat::{CombatSystem, Direction};
use super::renderer::{
    COLOR_BLACK, COLOR_BLUE, COLOR_BROWN, COLOR_CYAN, COLOR_DARK_GRAY, COLOR_DUNGEON, COLOR_FOREST,
    COLOR_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, COLOR_YELLOW, TILE_SIZE, clear_screen,
    draw_hp_bar, draw_rect, draw_text, fill_rect,
};
use crate::data::{Map, Npc, Tile};

pub fn draw_explore(
    fb: &mut Framebuffer,
    map: &Map,
    player: &Player,
    combat: &CombatSystem,
    npcs: &[Npc],
) {
    clear_screen(fb);
    let screen_h = fb.height() as i32;
    draw_map_with_entities(fb, map, player, combat, npcs, screen_h);
    draw_hud(fb, map, player, combat, screen_h);
}

fn draw_map_with_entities(
    fb: &mut Framebuffer,
    map: &Map,
    player: &Player,
    combat: &CombatSystem,
    npcs: &[Npc],
    screen_h: i32,
) {
    let screen_w = fb.width() as i32;
    let view_tiles_x = (screen_w / TILE_SIZE) as usize;
    let view_tiles_y = ((screen_h - 20) / TILE_SIZE) as usize;

    let half_x = view_tiles_x / 2;
    let half_y = view_tiles_y / 2;

    let camera_x = player.x as i32 - half_x as i32;
    let camera_y = player.y as i32 - half_y as i32;

    for screen_y in 0..view_tiles_y {
        for screen_x in 0..view_tiles_x {
            let map_x = camera_x + screen_x as i32;
            let map_y = camera_y + screen_y as i32;

            let px = (screen_x as i32) * TILE_SIZE;
            let py = (screen_y as i32) * TILE_SIZE;

            if map_x < 0 || map_y < 0 || map_x >= map.width as i32 || map_y >= map.height as i32 {
                fill_rect(fb, px, py, TILE_SIZE, TILE_SIZE, COLOR_BLACK);
            } else {
                let tile = map.get_tile(map_x as usize, map_y as usize);
                fill_rect(fb, px, py, TILE_SIZE, TILE_SIZE, tile_color(tile));
            }
        }
    }

    for npc in npcs {
        if npc.map_id != player.current_map_id {
            continue;
        }

        let screen_x = npc.x as i32 - camera_x;
        let screen_y = npc.y as i32 - camera_y;

        if screen_x >= 0
            && screen_y >= 0
            && screen_x < view_tiles_x as i32
            && screen_y < view_tiles_y as i32
        {
            let px = screen_x * TILE_SIZE;
            let py = screen_y * TILE_SIZE;
            fill_rect(fb, px + 1, py + 1, TILE_SIZE - 2, TILE_SIZE - 2, COLOR_CYAN);
        }
    }

    for enemy in &combat.enemies {
        if enemy.is_dead() {
            continue;
        }

        let screen_x = enemy.x as i32 - camera_x;
        let screen_y = enemy.y as i32 - camera_y;

        if screen_x >= 0
            && screen_y >= 0
            && screen_x < view_tiles_x as i32
            && screen_y < view_tiles_y as i32
        {
            let px = screen_x * TILE_SIZE;
            let py = screen_y * TILE_SIZE;

            let enemy_color = if enemy.hit_flash > 0 {
                COLOR_WHITE
            } else {
                COLOR_RED
            };

            fill_rect(
                fb,
                px + 1,
                py + 1,
                TILE_SIZE - 2,
                TILE_SIZE - 2,
                enemy_color,
            );
        }
    }

    let px = (half_x as i32) * TILE_SIZE;
    let py = (half_y as i32) * TILE_SIZE;

    let player_color = if combat.player_hit_flash > 0 {
        COLOR_RED
    } else {
        COLOR_WHITE
    };
    fill_rect(
        fb,
        px + 1,
        py + 1,
        TILE_SIZE - 2,
        TILE_SIZE - 2,
        player_color,
    );

    draw_facing_indicator(fb, half_x as i32, half_y as i32, &player.facing);
}

fn draw_facing_indicator(fb: &mut Framebuffer, screen_x: i32, screen_y: i32, facing: &Direction) {
    let (ox, oy, w, h) = match facing {
        Direction::Up => (TILE_SIZE / 2 - 1, 0, 2, 2),
        Direction::Down => (TILE_SIZE / 2 - 1, TILE_SIZE - 2, 2, 2),
        Direction::Left => (0, TILE_SIZE / 2 - 1, 2, 2),
        Direction::Right => (TILE_SIZE - 2, TILE_SIZE / 2 - 1, 2, 2),
    };

    let px = screen_x * TILE_SIZE + ox;
    let py = screen_y * TILE_SIZE + oy;

    fill_rect(fb, px, py, w, h, COLOR_YELLOW);
}

fn draw_hud(
    fb: &mut Framebuffer,
    map: &Map,
    player: &Player,
    combat: &CombatSystem,
    screen_h: i32,
) {
    let screen_w = fb.width() as i32;
    let hud_y = screen_h - 20;

    fill_rect(fb, 0, hud_y, screen_w, 20, COLOR_BLACK);
    draw_rect(fb, 0, hud_y, screen_w, 20, COLOR_WHITE);

    draw_text(fb, 4, hud_y + 2, &map.name, COLOR_CYAN);

    draw_hp_bar(
        fb,
        4,
        hud_y + 12,
        40,
        player.stats.current_hp,
        player.stats.max_hp,
    );

    let hp_text = format!("{}/{}", player.stats.current_hp, player.stats.max_hp);
    draw_text(fb, 46, hud_y + 10, &hp_text, COLOR_WHITE);

    let lv_text = format!("Lv{}", player.stats.level);
    draw_text(fb, screen_w - 30, hud_y + 2, &lv_text, COLOR_YELLOW);

    if let Some(enemy) = combat.enemies.iter().find(|e| !e.is_dead()) {
        draw_text(fb, screen_w - 50, hud_y + 10, &enemy.data.name, COLOR_RED);
    }
}

fn tile_color(tile: Tile) -> Color {
    match tile {
        Tile::Wall => COLOR_DARK_GRAY,
        Tile::Floor | Tile::PlayerStart | Tile::Enemy => COLOR_GRAY,
        Tile::House => COLOR_BROWN,
        Tile::Dungeon => COLOR_DUNGEON,
        Tile::Treasure => COLOR_YELLOW,
        Tile::Exit => COLOR_GREEN,
        Tile::Water => COLOR_BLUE,
        Tile::Tree => COLOR_FOREST,
    }
}

pub fn check_tile_event(map: &Map, player: &Player) -> Option<TileEvent> {
    let tile = map.get_tile(player.x, player.y);

    match tile {
        Tile::Treasure => Some(TileEvent::Treasure),
        Tile::Exit => {
            for (ex, ey, target) in &map.exits {
                if *ex == player.x && *ey == player.y {
                    return Some(TileEvent::MapExit(target.clone()));
                }
            }
            None
        }
        Tile::House => Some(TileEvent::Npc),
        Tile::Dungeon => {
            for (dx, dy, target) in &map.dungeons {
                if *dx == player.x && *dy == player.y {
                    return Some(TileEvent::DungeonEntrance(target.clone()));
                }
            }
            None
        }
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum TileEvent {
    Treasure,
    MapExit(alloc::string::String),
    Npc,
    DungeonEntrance(alloc::string::String),
}

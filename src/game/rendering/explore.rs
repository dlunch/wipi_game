use alloc::format;
use alloc::string::String;

use wipi::framebuffer::{Color, Framebuffer};

use super::renderer::{
    COLOR_BLACK, COLOR_BLUE, COLOR_BROWN, COLOR_CYAN, COLOR_DARK_GRAY, COLOR_DUNGEON, COLOR_FOREST,
    COLOR_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, COLOR_YELLOW, TILE_SIZE, clear_screen,
    draw_rect, draw_text, fill_rect, truncate_by_chars,
};
use super::sprites::SpriteAtlas;
use crate::data::{Direction, Map, SkillType, Tile};
use crate::game::ExploreAction;
use crate::game::ExploreRender;

const HUD_HEIGHT: i32 = 40;
const MINIMAP_RADIUS: i32 = 6;
const MINIMAP_CELL: i32 = 3;

pub fn draw_explore(fb: &mut Framebuffer, state: &ExploreRender, sprites: &SpriteAtlas) {
    let Ok(map) = state.data.find_map(&state.map_id) else {
        clear_screen(fb);
        draw_text(fb, 16, 16, "ERR: Map not found", COLOR_RED);
        return;
    };

    clear_screen(fb);
    let screen_h = fb.height() as i32;
    draw_map_with_entities(fb, map, state, sprites, screen_h);
    draw_minimap(fb, map, state);
    draw_hud(fb, map.name.as_str(), state, screen_h);
    draw_quest_notice(fb, state.quest_notice_timer);
}

fn draw_quest_notice(fb: &mut Framebuffer, timer: u32) {
    if timer == 0 {
        return;
    }

    let screen_w = fb.width() as i32;
    let box_w = 140;
    let box_h = 18;
    let x = (screen_w - box_w) / 2;
    let y = 8;

    fill_rect(fb, x, y, box_w, box_h, COLOR_BLACK);
    draw_rect(fb, x, y, box_w, box_h, COLOR_YELLOW);
    draw_text(fb, x + 8, y + 5, "Quest Accepted", COLOR_GREEN);
}

fn draw_map_with_entities(
    fb: &mut Framebuffer,
    map: &Map,
    state: &ExploreRender,
    sprites: &SpriteAtlas,
    screen_h: i32,
) {
    let screen_w = fb.width() as i32;
    let view_tiles_x = (screen_w / TILE_SIZE) as usize;
    let view_tiles_y = ((screen_h - HUD_HEIGHT) / TILE_SIZE) as usize;

    let half_x = view_tiles_x / 2;
    let half_y = view_tiles_y / 2;

    let camera_x = state.player_x as i32 - half_x as i32;
    let camera_y = state.player_y as i32 - half_y as i32;

    for screen_y in 0..view_tiles_y {
        for screen_x in 0..view_tiles_x {
            let map_x = camera_x + screen_x as i32;
            let map_y = camera_y + screen_y as i32;

            let px = (screen_x as i32) * TILE_SIZE;
            let py = (screen_y as i32) * TILE_SIZE;

            if map_x < 0 || map_y < 0 || map_x >= map.width as i32 || map_y >= map.height as i32 {
                fill_rect(fb, px, py, TILE_SIZE, TILE_SIZE, COLOR_BLACK);
            } else {
                let map_xu = map_x as usize;
                let map_yu = map_y as usize;
                let tile = map.get_tile(map_xu, map_yu);
                let is_opened = tile == Tile::Treasure
                    && is_treasure_opened(&state.opened_treasures, &state.map_id, map_xu, map_yu);
                fill_rect(
                    fb,
                    px,
                    py,
                    TILE_SIZE,
                    TILE_SIZE,
                    tile_color(tile, is_opened),
                );
            }
        }
    }

    for npc in &state.data.npcs {
        if npc.map_id != state.map_id {
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
            if let Some((npc_sprite, frame)) = sprites.npc_frame(state.anim_tick) {
                fb.draw_image(px, py, frame.w, frame.h, npc_sprite, frame.sx, frame.sy);
            } else {
                fill_rect(fb, px + 1, py + 1, TILE_SIZE - 2, TILE_SIZE - 2, COLOR_CYAN);
            }

            let dist = state.player_x.abs_diff(npc.x) + state.player_y.abs_diff(npc.y);
            if dist <= 2 {
                draw_text(fb, px, py - 8, &npc.name, COLOR_YELLOW);
            }
        }
    }

    for enemy in &state.enemies {
        if enemy.dead {
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

            if let Some((enemy_sprite, frame)) = sprites.enemy_frame(state.anim_tick) {
                fb.draw_image(px, py, frame.w, frame.h, enemy_sprite, frame.sx, frame.sy);
                if enemy.hit_flash > 0 {
                    draw_rect(fb, px, py, TILE_SIZE, TILE_SIZE, COLOR_WHITE);
                }
            } else {
                fill_rect(
                    fb,
                    px + 1,
                    py + 1,
                    TILE_SIZE - 2,
                    TILE_SIZE - 2,
                    enemy_color,
                );
            }

            let bar_width = if enemy.max_hp > 0 {
                enemy.hp.max(0) * (TILE_SIZE - 2) / enemy.max_hp
            } else {
                0
            };
            fill_rect(fb, px + 1, py - 2, TILE_SIZE - 2, 2, COLOR_DARK_GRAY);
            fill_rect(fb, px + 1, py - 2, bar_width, 2, COLOR_GREEN);

            let near_player =
                enemy.x.abs_diff(state.player_x) + enemy.y.abs_diff(state.player_y) <= 1;
            if enemy.attack_cooldown == 0 && near_player {
                draw_text(fb, px + (TILE_SIZE / 2) - 2, py - 10, "!", COLOR_RED);
            }
        }
    }

    let px = (half_x as i32) * TILE_SIZE;
    let py = (half_y as i32) * TILE_SIZE;

    let player_color = if state.player_hit_flash > 0 {
        COLOR_RED
    } else {
        COLOR_WHITE
    };
    if let Some((player_sprite, frame)) =
        sprites.player_frame(state.player_facing, state.player_moving, state.anim_tick)
    {
        fb.draw_image(px, py, frame.w, frame.h, player_sprite, frame.sx, frame.sy);
        if state.player_hit_flash > 0 {
            draw_rect(fb, px, py, TILE_SIZE, TILE_SIZE, COLOR_RED);
        }
    } else {
        fill_rect(
            fb,
            px + 1,
            py + 1,
            TILE_SIZE - 2,
            TILE_SIZE - 2,
            player_color,
        );
    }

    let hp_bar_width = ((state.hp * (TILE_SIZE - 2) as u32) / state.max_hp) as i32;
    fill_rect(
        fb,
        px + 1,
        py + TILE_SIZE,
        TILE_SIZE - 2,
        2,
        COLOR_DARK_GRAY,
    );
    fill_rect(fb, px + 1, py + TILE_SIZE, hp_bar_width, 2, COLOR_GREEN);

    let mp_bar_width = ((state.mp * (TILE_SIZE - 2) as u32) / state.max_mp) as i32;
    fill_rect(
        fb,
        px + 1,
        py + TILE_SIZE + 2,
        TILE_SIZE - 2,
        2,
        COLOR_DARK_GRAY,
    );
    fill_rect(fb, px + 1, py + TILE_SIZE + 2, mp_bar_width, 2, COLOR_BLUE);

    draw_facing_indicator(fb, half_x as i32, half_y as i32, state.player_facing);

    for effect in &state.skill_effects {
        let screen_x = effect.x as i32 - camera_x;
        let screen_y = effect.y as i32 - camera_y;

        if screen_x >= 0
            && screen_y >= 0
            && screen_x < view_tiles_x as i32
            && screen_y < view_tiles_y as i32
        {
            let px = screen_x * TILE_SIZE;
            let py = screen_y * TILE_SIZE;

            let color = match effect.effect_type {
                SkillType::Ranged => COLOR_YELLOW,
                SkillType::Heal => COLOR_GREEN,
                SkillType::Area => COLOR_CYAN,
            };
            draw_rect(fb, px, py, TILE_SIZE, TILE_SIZE, color);
        }
    }
}

fn is_treasure_opened(opened: &[(String, usize, usize)], map_id: &str, x: usize, y: usize) -> bool {
    opened
        .iter()
        .any(|(opened_map_id, tx, ty)| opened_map_id == map_id && *tx == x && *ty == y)
}

fn draw_facing_indicator(fb: &mut Framebuffer, screen_x: i32, screen_y: i32, facing: Direction) {
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

fn draw_hud(fb: &mut Framebuffer, map_name: &str, state: &ExploreRender, screen_h: i32) {
    let screen_w = fb.width() as i32;
    let hud_y = screen_h - HUD_HEIGHT;

    fill_rect(fb, 0, hud_y, screen_w, HUD_HEIGHT, COLOR_BLACK);
    draw_rect(fb, 0, hud_y, screen_w, HUD_HEIGHT, COLOR_WHITE);

    draw_text(fb, 4, hud_y + 2, map_name, COLOR_CYAN);

    if state.active_quest_count > 0 {
        let quest_text = format!("Q:{}", state.active_quest_count);
        draw_text(fb, screen_w - 70, hud_y + 2, &quest_text, COLOR_GREEN);
    }

    let lv_text = format!("Lv{}", state.level);
    draw_text(fb, screen_w - 30, hud_y + 2, &lv_text, COLOR_YELLOW);

    if let Some(enemy_name) = &state.first_live_enemy_name {
        draw_text(fb, screen_w - 80, hud_y + 2, enemy_name, COLOR_RED);
    }

    if let Some(quest) = &state.tracked_quest {
        let progress = format!(
            "{} {}/{}",
            quest.name, quest.current_count, quest.target_count
        );
        let progress = truncate_by_chars(&progress, 24);
        let color = if quest.completed {
            COLOR_GREEN
        } else {
            COLOR_YELLOW
        };
        draw_text(fb, 4, hud_y + 12, progress, color);
    }

    if let Some(hint) = &state.interaction_hint {
        let hint = truncate_by_chars(hint, 24);
        draw_text(fb, 4, hud_y + 20, hint, COLOR_CYAN);
    }

    let mut status = String::new();
    if state.player_status.poison_timer > 0 {
        status.push_str("PSN ");
    }
    if state.player_status.stun_timer > 0 {
        status.push_str("STN ");
    }
    if state.player_status.armor_break_timer > 0 {
        status.push_str("BRK ");
    }
    if !status.is_empty() {
        draw_text(fb, screen_w - 70, hud_y + 20, status.trim_end(), COLOR_RED);
    }

    if !state.peaceful {
        draw_skill_bar(
            fb,
            hud_y + 28,
            screen_w,
            state.mp,
            &state.skill_cooldowns,
            &state.key_actions,
        );
    }
}

fn draw_minimap(fb: &mut Framebuffer, map: &Map, state: &ExploreRender) {
    let diam = MINIMAP_RADIUS * 2 + 1;
    let box_w = diam * MINIMAP_CELL + 4;
    let box_h = diam * MINIMAP_CELL + 4;
    let base_x = fb.width() as i32 - box_w - 4;
    let base_y = 4;

    fill_rect(fb, base_x, base_y, box_w, box_h, COLOR_BLACK);
    draw_rect(fb, base_x, base_y, box_w, box_h, COLOR_WHITE);

    for dy in -MINIMAP_RADIUS..=MINIMAP_RADIUS {
        for dx in -MINIMAP_RADIUS..=MINIMAP_RADIUS {
            let mx = state.player_x as i32 + dx;
            let my = state.player_y as i32 + dy;
            let px = base_x + 2 + (dx + MINIMAP_RADIUS) * MINIMAP_CELL;
            let py = base_y + 2 + (dy + MINIMAP_RADIUS) * MINIMAP_CELL;
            let color = if mx < 0 || my < 0 || mx >= map.width as i32 || my >= map.height as i32 {
                COLOR_BLACK
            } else {
                match map.get_tile(mx as usize, my as usize) {
                    Tile::Wall => COLOR_DARK_GRAY,
                    Tile::Water => COLOR_BLUE,
                    Tile::Tree => COLOR_FOREST,
                    Tile::Dungeon => COLOR_DUNGEON,
                    Tile::Treasure => COLOR_YELLOW,
                    Tile::Exit => COLOR_GREEN,
                    _ => COLOR_GRAY,
                }
            };
            fill_rect(fb, px, py, MINIMAP_CELL, MINIMAP_CELL, color);
        }
    }

    for npc in &state.data.npcs {
        if npc.map_id != state.map_id {
            continue;
        }
        let dx = npc.x as i32 - state.player_x as i32;
        let dy = npc.y as i32 - state.player_y as i32;
        if dx.abs() > MINIMAP_RADIUS || dy.abs() > MINIMAP_RADIUS {
            continue;
        }
        let px = base_x + 2 + (dx + MINIMAP_RADIUS) * MINIMAP_CELL;
        let py = base_y + 2 + (dy + MINIMAP_RADIUS) * MINIMAP_CELL;
        fill_rect(fb, px, py, MINIMAP_CELL, MINIMAP_CELL, COLOR_CYAN);
    }

    for enemy in &state.enemies {
        if enemy.dead {
            continue;
        }
        let dx = enemy.x as i32 - state.player_x as i32;
        let dy = enemy.y as i32 - state.player_y as i32;
        if dx.abs() > MINIMAP_RADIUS || dy.abs() > MINIMAP_RADIUS {
            continue;
        }
        let px = base_x + 2 + (dx + MINIMAP_RADIUS) * MINIMAP_CELL;
        let py = base_y + 2 + (dy + MINIMAP_RADIUS) * MINIMAP_CELL;
        fill_rect(fb, px, py, MINIMAP_CELL, MINIMAP_CELL, COLOR_RED);
    }

    let center_x = base_x + 2 + MINIMAP_RADIUS * MINIMAP_CELL;
    let center_y = base_y + 2 + MINIMAP_RADIUS * MINIMAP_CELL;
    fill_rect(
        fb,
        center_x,
        center_y,
        MINIMAP_CELL,
        MINIMAP_CELL,
        COLOR_WHITE,
    );
}

fn draw_skill_bar(
    fb: &mut Framebuffer,
    y: i32,
    screen_w: i32,
    current_mp: u32,
    skill_cooldowns: &[u32; 3],
    key_actions: &[Option<ExploreAction>; 3],
) {
    let slot_width = screen_w / 3;

    for (i, action_opt) in key_actions.iter().enumerate() {
        let x = i as i32 * slot_width + 4;
        let key = match i {
            0 => "1",
            1 => "2",
            _ => "3",
        };

        let Some(action) = action_opt else {
            draw_text(fb, x, y, &format!("[{}]-", key), COLOR_GRAY);
            continue;
        };

        let (cd, mp_cost) = match action.skill() {
            Some((slot, skill)) => (skill_cooldowns[slot], skill.mp_cost.max(0) as u32),
            None => (0, 0),
        };

        let is_ready = cd == 0 && current_mp >= mp_cost;
        let color = if is_ready { COLOR_WHITE } else { COLOR_GRAY };
        let text = if cd > 0 {
            format!("[{}]{} {}", key, action.label(), cd / 10)
        } else {
            format!("[{}]{}", key, action.label())
        };
        draw_text(fb, x, y, &text, color);
    }
}

fn tile_color(tile: Tile, is_opened_treasure: bool) -> Color {
    match tile {
        Tile::Wall => COLOR_DARK_GRAY,
        Tile::Floor | Tile::PlayerStart | Tile::Enemy => COLOR_GRAY,
        Tile::House => COLOR_BROWN,
        Tile::Dungeon => COLOR_DUNGEON,
        Tile::Treasure => {
            if is_opened_treasure {
                COLOR_BROWN
            } else {
                COLOR_YELLOW
            }
        }
        Tile::Exit => COLOR_GREEN,
        Tile::Water => COLOR_BLUE,
        Tile::Tree => COLOR_FOREST,
    }
}

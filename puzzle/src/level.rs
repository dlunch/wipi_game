use alloc::vec::Vec;

use anyhow::{Result, bail, ensure};

// 24px tiles on a 240x320 screen, minus the HUD rows.
pub const MAX_WIDTH: usize = 10;
pub const MAX_HEIGHT: usize = 11;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Goal,
}

pub struct Level {
    pub width: usize,
    pub tiles: Vec<Tile>,
    pub player: usize,
    pub boxes: Vec<usize>,
}

/// Levels are blank-line separated grids: `#` wall, `@` player, `$` box, `.` goal,
/// `*` box on goal, `+` player on goal, space floor.
pub fn parse_levels(text: &str) -> Result<Vec<Level>> {
    let mut levels = Vec::new();
    let mut rows = Vec::new();
    for line in text.lines().chain(core::iter::once("")) {
        let line = line.trim_end();
        if !line.is_empty() {
            rows.push(line);
        } else if !rows.is_empty() {
            levels.push(parse_level(levels.len() + 1, &rows)?);
            rows.clear();
        }
    }
    ensure!(!levels.is_empty(), "no levels");

    Ok(levels)
}

fn parse_level(number: usize, rows: &[&str]) -> Result<Level> {
    let width = rows[0].len();
    let height = rows.len();
    ensure!(
        width <= MAX_WIDTH && height <= MAX_HEIGHT,
        "level {number}: larger than {MAX_WIDTH}x{MAX_HEIGHT}"
    );

    let mut tiles = Vec::with_capacity(width * height);
    let mut player = None;
    let mut boxes = Vec::new();
    let mut goals = 0;
    for (y, row) in rows.iter().enumerate() {
        ensure!(row.len() == width, "level {number}: uneven row width");
        for (x, c) in row.bytes().enumerate() {
            // A closed wall border keeps every move inside the grid.
            let border = x == 0 || y == 0 || x == width - 1 || y == height - 1;
            ensure!(c == b'#' || !border, "level {number}: open border");

            let tile = match c {
                b'#' => Tile::Wall,
                b' ' | b'@' | b'$' => Tile::Floor,
                b'.' | b'+' | b'*' => Tile::Goal,
                _ => bail!("level {number}: unknown tile '{}'", c as char),
            };
            if matches!(c, b'@' | b'+') {
                ensure!(
                    player.replace(tiles.len()).is_none(),
                    "level {number}: multiple players"
                );
            }
            if matches!(c, b'$' | b'*') {
                boxes.push(tiles.len());
            }
            if tile == Tile::Goal {
                goals += 1;
            }
            tiles.push(tile);
        }
    }
    let Some(player) = player else {
        bail!("level {number}: no player");
    };
    ensure!(
        !boxes.is_empty() && boxes.len() == goals,
        "level {number}: box and goal counts differ"
    );

    Ok(Level {
        width,
        tiles,
        player,
        boxes,
    })
}

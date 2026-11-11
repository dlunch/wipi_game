use alloc::vec::Vec;

use crate::level::{Level, Tile};

#[derive(Clone, Copy)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

pub struct Game {
    pub levels: Vec<Level>,
    pub current: usize,
    pub player: usize,
    pub boxes: Vec<usize>,
    /// Each move with whether it pushed a box, for undo.
    pub history: Vec<(Dir, bool)>,
}

impl Game {
    pub fn new(levels: Vec<Level>, current: usize) -> Self {
        let mut game = Self {
            // A saved stage may no longer exist after the level set changes.
            current: if current < levels.len() { current } else { 0 },
            levels,
            player: 0,
            boxes: Vec::new(),
            history: Vec::new(),
        };
        game.restart();
        game
    }

    pub fn restart(&mut self) {
        let level = &self.levels[self.current];
        self.player = level.player;
        self.boxes.clone_from(&level.boxes);
        self.history.clear();
    }

    pub fn change_level(&mut self, forward: bool) {
        let count = self.levels.len();
        self.current = (self.current + if forward { 1 } else { count - 1 }) % count;
        self.restart();
    }

    pub fn is_solved(&self) -> bool {
        let level = &self.levels[self.current];
        self.boxes.iter().all(|&b| level.tiles[b] == Tile::Goal)
    }

    /// Returns whether the player moved.
    pub fn step(&mut self, dir: Dir) -> bool {
        if self.is_solved() {
            return false;
        }
        let level = &self.levels[self.current];
        let offset = self.offset(dir);
        let next = self.player.wrapping_add_signed(offset);
        if level.tiles[next] == Tile::Wall {
            return false;
        }
        let pushed = self.boxes.iter().position(|&b| b == next);
        if let Some(pushed) = pushed {
            let beyond = next.wrapping_add_signed(offset);
            if level.tiles[beyond] == Tile::Wall || self.boxes.contains(&beyond) {
                return false;
            }
            self.boxes[pushed] = beyond;
        }
        self.player = next;
        self.history.push((dir, pushed.is_some()));
        true
    }

    pub fn undo(&mut self) {
        let Some((dir, pushed)) = self.history.pop() else {
            return;
        };
        let offset = self.offset(dir);
        if pushed {
            let pushed_to = self.player.wrapping_add_signed(offset);
            for b in &mut self.boxes {
                if *b == pushed_to {
                    *b = self.player;
                }
            }
        }
        self.player = self.player.wrapping_add_signed(-offset);
    }

    fn offset(&self, dir: Dir) -> isize {
        let width = self.levels[self.current].width as isize;
        match dir {
            Dir::Up => -width,
            Dir::Down => width,
            Dir::Left => -1,
            Dir::Right => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use super::{Dir, Game};
    use crate::level::parse_levels;

    #[test]
    fn bundled_levels_are_solved_by_recorded_solutions() -> Result<()> {
        let mut game = Game::new(parse_levels(include_str!("../resources/levels.dat"))?, 0);
        let solutions = include_str!("../tests/solutions.txt");
        ensure!(solutions.lines().count() == game.levels.len());

        for solution in solutions.lines() {
            assert!(!game.is_solved());
            for c in solution.chars() {
                // Solutions are move-optimal, so no move may be rejected.
                assert!(game.step(match c {
                    'U' => Dir::Up,
                    'D' => Dir::Down,
                    'L' => Dir::Left,
                    _ => Dir::Right,
                }));
            }
            assert!(game.is_solved(), "level {}", game.current + 1);
            assert!(!game.step(Dir::Up));

            while !game.history.is_empty() {
                game.undo();
            }
            let level = &game.levels[game.current];
            assert_eq!(game.player, level.player);
            assert_eq!(game.boxes, level.boxes);

            game.change_level(true);
        }
        assert_eq!(game.current, 0);

        Ok(())
    }

    #[test]
    fn malformed_levels_are_rejected() {
        for text in [
            "",
            "####\n#@$#\n####",
            "####\n#$.#\n####",
            "#####\n#@$. \n#####",
            "#####\n#@$.#\n####",
            "#####\n#@$x#\n#####",
            "######\n#@@$.#\n######",
            "#############\n#@$        .#\n#############",
        ] {
            assert!(parse_levels(text).is_err(), "{text:?}");
        }
    }
}

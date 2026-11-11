#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod game;
mod level;
mod render;
mod save;

use alloc::rc::Rc;
use core::{cell::RefCell, time::Duration};

use anyhow::{Result, anyhow};
use wipi::{
    app::App, event::KeyCode, framebuffer::Framebuffer, graphics::repaint, image::Image,
    resource::Resource, timer::Timer, wipi_main,
};

use crate::game::{Dir, Game};

const REPEAT_INTERVAL: Duration = Duration::from_millis(100);
// Timer ticks a direction key stays held before it starts repeating.
const REPEAT_DELAY_TICKS: u32 = 3;

struct Session {
    game: Game,
    tiles: Image,
    held: Option<(KeyCode, Dir)>,
    held_ticks: u32,
}

impl Session {
    fn load() -> Result<Self> {
        let levels = Resource::new("levels.dat").map_err(|e| anyhow!("levels.dat: {:?}", e))?;
        let levels = level::parse_levels(core::str::from_utf8(levels.read())?)?;
        let tiles = Image::new("tiles.png").map_err(|e| anyhow!("tiles.png: {:?}", e))?;

        Ok(Self {
            game: Game::new(levels, save::load_stage()?),
            tiles,
            held: None,
            held_ticks: 0,
        })
    }

    /// Returns whether the screen changed.
    fn on_keydown(&mut self, key: KeyCode) -> Result<bool> {
        let dir = match key {
            KeyCode::Up | KeyCode::Key2 => Some(Dir::Up),
            KeyCode::Down | KeyCode::Key8 => Some(Dir::Down),
            KeyCode::Left | KeyCode::Key4 => Some(Dir::Left),
            KeyCode::Right | KeyCode::Key6 => Some(Dir::Right),
            _ => None,
        };
        if let Some(dir) = dir {
            self.held = Some((key, dir));
            self.held_ticks = 0;
            return Ok(self.game.step(dir));
        }

        match key {
            KeyCode::Key1 | KeyCode::Back => self.game.undo(),
            KeyCode::Key3 => self.game.restart(),
            KeyCode::Key7 => self.change_level(false)?,
            KeyCode::Key9 => self.change_level(true)?,
            KeyCode::Ok if self.game.is_solved() => self.change_level(true)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn change_level(&mut self, forward: bool) -> Result<()> {
        self.game.change_level(forward);
        save::save_stage(self.game.current)
    }

    /// Returns whether the held direction key moved the player.
    fn tick(&mut self) -> bool {
        let Some((_, dir)) = self.held else {
            return false;
        };
        self.held_ticks += 1;
        self.held_ticks > REPEAT_DELAY_TICKS && self.game.step(dir)
    }
}

pub struct PuzzleGame {
    session: Rc<RefCell<Result<Session>>>,
    _timer: Timer,
}

impl PuzzleGame {
    fn new() -> Self {
        let session = Rc::new(RefCell::new(Session::load()));

        let timer_session = Rc::clone(&session);
        let timer = Timer::periodic(REPEAT_INTERVAL, move || {
            // The borrow must end before repaint, which may paint right away.
            let moved = match &mut *timer_session.borrow_mut() {
                Ok(session) => session.tick(),
                Err(_) => false,
            };
            if moved {
                repaint(0, 0, 0, 240, 320);
            }
        });

        Self {
            session,
            _timer: timer,
        }
    }
}

impl App for PuzzleGame {
    fn on_paint(&mut self) {
        let mut fb = Framebuffer::screen_framebuffer();
        match &*self.session.borrow() {
            Ok(session) => render::render(&session.game, &session.tiles, &mut fb),
            Err(error) => render::render_error(error, &mut fb),
        }
    }

    fn on_keydown(&mut self, key: KeyCode) {
        {
            let mut session = self.session.borrow_mut();
            let Ok(current) = &mut *session else {
                return;
            };
            match current.on_keydown(key) {
                Ok(false) => return,
                Ok(true) => {}
                Err(error) => *session = Err(error),
            }
        }
        repaint(0, 0, 0, 240, 320);
    }

    fn on_keyup(&mut self, key: KeyCode) {
        if let Ok(session) = &mut *self.session.borrow_mut()
            && matches!(session.held, Some((held, _)) if held == key)
        {
            session.held = None;
        }
    }
}

#[wipi_main]
pub fn main() -> PuzzleGame {
    PuzzleGame::new()
}

#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod engine;
mod game;

use alloc::rc::Rc;
use core::{cell::RefCell, time::Duration};

use wipi::{
    app::App, event::KeyCode, framebuffer::Framebuffer, graphics::repaint, timer::Timer, wipi_main,
};

use crate::{
    engine::GameEngine,
    game::{rendering::render, ui::state::InputKey},
};

fn map_key(key: KeyCode) -> Option<InputKey> {
    match key {
        KeyCode::Ok => Some(InputKey::Ok),
        KeyCode::Back => Some(InputKey::Back),
        KeyCode::Up => Some(InputKey::Up),
        KeyCode::Down => Some(InputKey::Down),
        KeyCode::Left => Some(InputKey::Left),
        KeyCode::Right => Some(InputKey::Right),
        KeyCode::Key0 => Some(InputKey::Key0),
        KeyCode::Key1 => Some(InputKey::Key1),
        KeyCode::Key2 => Some(InputKey::Key2),
        KeyCode::Key3 => Some(InputKey::Key3),
        KeyCode::Key4 => Some(InputKey::Key4),
        KeyCode::Key5 => Some(InputKey::Key5),
        KeyCode::Key6 => Some(InputKey::Key6),
        KeyCode::Key7 => Some(InputKey::Key7),
        KeyCode::Key8 => Some(InputKey::Key8),
        KeyCode::Key9 => Some(InputKey::Key9),
        _ => None,
    }
}

pub struct RpgGame {
    engine: Rc<RefCell<GameEngine>>,
    _timer: Timer,
}

impl Default for RpgGame {
    fn default() -> Self {
        Self::new()
    }
}

impl RpgGame {
    fn tick(engine: &Rc<RefCell<GameEngine>>) {
        if engine.borrow_mut().tick() {
            repaint(0, 0, 0, 240, 320);
        }
    }

    pub fn new() -> Self {
        let engine = Rc::new(RefCell::new(GameEngine::new()));

        let timer_engine = Rc::clone(&engine);
        let timer = Timer::periodic(Duration::from_millis(33), move || {
            Self::tick(&timer_engine);
        });

        Self {
            engine,
            _timer: timer,
        }
    }
}

impl App for RpgGame {
    fn on_paint(&mut self) {
        let engine = self.engine.borrow();
        let mut fb = Framebuffer::screen_framebuffer();
        render(
            engine.render_state(),
            engine.sprite_atlas(),
            engine.render_fx(),
            &mut fb,
        );
    }

    fn on_keydown(&mut self, key: KeyCode) {
        if let Some(key) = map_key(key) {
            self.engine.borrow_mut().on_keydown(key);
        }
    }

    fn on_keyup(&mut self, key: KeyCode) {
        if let Some(key) = map_key(key) {
            self.engine.borrow_mut().on_keyup(key);
        }
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}

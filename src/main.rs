#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;
mod runtime;

use alloc::rc::Rc;
use core::cell::RefCell;
use core::time::Duration;

use wipi::app::App;
use wipi::event::KeyCode;
use wipi::framebuffer::Framebuffer;
use wipi::graphics::repaint;
use wipi::timer::Timer;
use wipi::wipi_main;

use crate::game::{InputKey, RenderState, render};
use crate::runtime::GameRuntime;

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
    runtime: Rc<RefCell<GameRuntime>>,
    render_state: Rc<RefCell<RenderState>>,
    _timer: Timer,
}

impl Default for RpgGame {
    fn default() -> Self {
        Self::new()
    }
}

impl RpgGame {
    fn tick(runtime: &Rc<RefCell<GameRuntime>>, render_state: &Rc<RefCell<RenderState>>) {
        let mut runtime = runtime.borrow_mut();
        let rs = runtime.tick_and_build_render_state();
        *render_state.borrow_mut() = rs;
        drop(runtime);
        repaint(0, 0, 0, 240, 320);
    }

    pub fn new() -> Self {
        let runtime = Rc::new(RefCell::new(GameRuntime::new()));
        let render_state = Rc::new(RefCell::new(RenderState::Loading { step: 0 }));

        let timer_runtime = Rc::clone(&runtime);
        let timer_render_state = Rc::clone(&render_state);
        let timer = Timer::periodic(Duration::from_millis(33), move || {
            Self::tick(&timer_runtime, &timer_render_state);
        });

        Self {
            runtime,
            render_state,
            _timer: timer,
        }
    }
}

impl App for RpgGame {
    fn on_paint(&mut self) {
        let render_state = self.render_state.borrow();
        let mut fb = Framebuffer::screen_framebuffer();
        render(&render_state, &mut fb);
    }

    fn on_keydown(&mut self, key: KeyCode) {
        if let Some(key) = map_key(key) {
            self.runtime.borrow_mut().on_keydown(key);
        }
    }

    fn on_keyup(&mut self, key: KeyCode) {
        if let Some(key) = map_key(key) {
            self.runtime.borrow_mut().on_keyup(key);
        }
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}

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

use crate::game::{RenderState, render};
use crate::runtime::GameRuntime;

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
        self.runtime.borrow_mut().on_keydown(key);
    }

    fn on_keyup(&mut self, key: KeyCode) {
        self.runtime.borrow_mut().on_keyup(key);
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}

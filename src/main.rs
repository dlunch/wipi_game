#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use wipi::{app::App, event::KeyCode, framebuffer::Framebuffer, graphics::repaint, wipi_main};

use game::RpgGame;
use game::app::AppAction;

impl App for RpgGame {
    fn on_paint(&mut self) {
        self.dispatch(AppAction::Tick);

        let mut fb = Framebuffer::screen_framebuffer();
        self.render(&mut fb);

        repaint(0, 0, 0, fb.width() as i32, fb.height() as i32);
    }

    fn on_keydown(&mut self, key: KeyCode) {
        self.dispatch(AppAction::KeyDown(key));
    }

    fn on_keyup(&mut self, key: KeyCode) {
        self.dispatch(AppAction::KeyUp(key));
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}

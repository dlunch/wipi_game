#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;

use wipi::app::App;
use wipi::event::KeyCode;
use wipi::framebuffer::Framebuffer;
use wipi::graphics::repaint;
use wipi::timer::Timer;
use wipi::wipi_main;

use crate::game::{
    AppAction, AppEffect, DialogIntent, ExploreIntent, GameData, GameState, InventoryIntent,
    MenuAction, MenuEvent, MenuIntent, MenuState, PauseMenuIntent, RenderState, SessionState,
    ShopIntent, build_render_state, has_save_data, render,
};

pub struct RpgGame {
    state: GameState,
    data: GameData,
    session: Option<SessionState>,
    timer: Option<Timer>,
}

impl Default for RpgGame {
    fn default() -> Self {
        Self::new()
    }
}

impl RpgGame {
    pub fn new() -> Self {
        Self {
            state: GameState::Loading(0),
            data: GameData::default(),
            session: None,
            timer: None,
        }
    }

    fn update(&mut self) {
        self.dispatch(AppAction::Tick);
    }

    fn collect_effects(&self, action: AppAction) -> Vec<AppEffect> {
        let mut effects = Vec::new();

        match action {
            AppAction::Tick => match self.state {
                GameState::Loading(_) => effects.push(AppEffect::UpdateLoading),
                GameState::Explore => {
                    effects.push(AppEffect::UpdateMovement);
                    effects.push(AppEffect::UpdateCombat);
                }
                _ => {}
            },
            AppAction::KeyDown(key) => match self.state {
                GameState::Loading(_) => {}
                GameState::Menu(_) => {
                    if let Some(intent) = MenuIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyMenuIntent(intent));
                    }
                }
                GameState::Explore => {
                    for intent in ExploreIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyExploreIntent(intent));
                    }
                }
                GameState::Inventory => {
                    if let Some(intent) = InventoryIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyInventoryIntent(intent));
                    }
                }
                GameState::Stats | GameState::QuestLog => {
                    if matches!(key, KeyCode::Back | KeyCode::Ok) {
                        effects.push(AppEffect::ReturnToExplore);
                    }
                }
                GameState::Dialog(_) => {
                    if let Some(intent) = DialogIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyDialogIntent(intent));
                    }
                }
                GameState::Shop(_) => {
                    if let Some(intent) = ShopIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyShopIntent(intent));
                    }
                }
                GameState::PauseMenu(_) => {
                    if let Some(intent) = PauseMenuIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyPauseMenuIntent(intent));
                    }
                }
                GameState::GameOver => {
                    if matches!(key, KeyCode::Ok) {
                        effects.push(AppEffect::ReturnToMenuFromGameOver);
                    }
                }
                GameState::Error(_) => {
                    if matches!(key, KeyCode::Ok) {
                        effects.push(AppEffect::Exit(1));
                    }
                }
            },
            AppAction::KeyUp(key) => effects.push(AppEffect::ReleaseMovementKey(key)),
        }

        effects
    }

    fn apply_effect(&mut self, effect: AppEffect) {
        match effect {
            AppEffect::UpdateLoading => {
                game::lifecycle::update_loading(&mut self.state, &mut self.data);
            }
            AppEffect::UpdateMovement => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::movement::update(
                    &self.state,
                    &mut s.movement,
                    &mut s.player,
                    &mut s.combat,
                    &self.data,
                );
            }
            AppEffect::UpdateCombat => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::combat::update_combat(
                    &mut self.state,
                    &mut s.player,
                    &mut s.skill_cooldowns,
                    &mut s.mp_regen_timer,
                    &mut s.combat,
                    &self.data,
                );
            }
            AppEffect::ApplyMenuIntent(intent) => {
                if let MenuEvent::Action(action) = game::menu::reduce(&mut self.state, intent) {
                    match action {
                        MenuAction::NewGame => {
                            let (state, session) = game::lifecycle::start_new_game(&self.data);
                            self.state = state;
                            self.session = Some(session);
                        }
                        MenuAction::Continue => {
                            let (state, session) = game::lifecycle::continue_game(&self.data);
                            self.state = state;
                            self.session = Some(session);
                        }
                        MenuAction::Exit => wipi::kernel::exit(0),
                    }
                }
            }
            AppEffect::ApplyExploreIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::explore::reduce(
                    &mut self.state,
                    &mut s.movement,
                    &mut s.player,
                    &mut s.skill_cooldowns,
                    &mut s.combat,
                    &self.data,
                    intent,
                );
            }
            AppEffect::ApplyInventoryIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::inventory::reduce(&mut self.state, &mut s.player, &mut s.inventory, intent);
            }
            AppEffect::ApplyDialogIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::dialog::reduce(&mut self.state, &mut s.player, &self.data, intent);
            }
            AppEffect::ApplyShopIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::shop::reduce(&mut self.state, &mut s.player, intent);
            }
            AppEffect::ApplyPauseMenuIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::menu::reduce_pause(&mut self.state, &s.player, &mut s.inventory, intent);
            }
            AppEffect::ReturnToExplore => self.state = GameState::Explore,
            AppEffect::ReturnToMenuFromGameOver => {
                self.state = GameState::Menu(MenuState::new(has_save_data()));
            }
            AppEffect::ReleaseMovementKey(key) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::movement::on_key_released(&mut s.movement, key);
            }
            AppEffect::Exit(code) => wipi::kernel::exit(code),
        }
    }

    fn dispatch(&mut self, action: AppAction) {
        let effects = self.collect_effects(action);
        for effect in effects {
            self.apply_effect(effect);
        }
    }

    fn render(&self, fb: &mut Framebuffer) {
        let render_state: RenderState<'_> =
            build_render_state(&self.state, self.session.as_ref(), &self.data);
        render(&render_state, fb);
    }

    fn ensure_timer(&mut self) {
        if self.timer.is_none() {
            self.timer = Some(Timer::periodic(Duration::from_millis(33), || {
                repaint(0, 0, 0, 240, 320);
            }));
        }
    }
}

impl App for RpgGame {
    fn on_paint(&mut self) {
        self.update();

        let mut fb = Framebuffer::screen_framebuffer();
        self.render(&mut fb);
        self.ensure_timer();
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

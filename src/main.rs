#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
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
    ShopIntent, UiState, build_render_state, has_save_data, render,
};

struct GameInner {
    state: GameState,
    data: Rc<GameData>,
    session: Option<SessionState>,
    ui: UiState,
}

impl GameInner {
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
                if let MenuEvent::Action(action) =
                    game::menu::reduce(&mut self.state, &mut self.ui.menu, intent)
                {
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
                let was_pause_menu = matches!(self.state, GameState::PauseMenu(_));
                let was_shop = matches!(self.state, GameState::Shop(_));
                game::explore::reduce(
                    &mut self.state,
                    &mut s.movement,
                    &mut s.player,
                    &mut s.skill_cooldowns,
                    &mut s.combat,
                    &self.data,
                    intent,
                );
                if !was_pause_menu && matches!(self.state, GameState::PauseMenu(_)) {
                    self.ui.pause_menu.selected = 0;
                }
                if matches!(self.state, GameState::Menu(_)) {
                    self.ui.menu.selected = 0;
                }
                if !was_shop && matches!(self.state, GameState::Shop(_)) {
                    self.ui.shop = game::ShopUiState::default();
                }
            }
            AppEffect::ApplyInventoryIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::inventory::reduce(
                    &mut self.state,
                    &mut s.player,
                    &mut self.ui.inventory,
                    intent,
                );
            }
            AppEffect::ApplyDialogIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                let was_shop = matches!(self.state, GameState::Shop(_));
                game::dialog::reduce(&mut self.state, &mut s.player, &self.data, intent);
                if !was_shop && matches!(self.state, GameState::Shop(_)) {
                    self.ui.shop = game::ShopUiState::default();
                }
            }
            AppEffect::ApplyShopIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::shop::reduce(&mut self.state, &mut s.player, &mut self.ui.shop, intent);
            }
            AppEffect::ApplyPauseMenuIntent(intent) => {
                let Some(s) = self.session.as_mut() else {
                    self.state = GameState::Error(String::from("No active session"));
                    return;
                };
                game::menu::reduce_pause(
                    &mut self.state,
                    &s.player,
                    &mut self.ui.pause_menu,
                    &mut self.ui.inventory,
                    &mut self.ui.shop,
                    intent,
                );
            }
            AppEffect::ReturnToExplore => self.state = GameState::Explore,
            AppEffect::ReturnToMenuFromGameOver => {
                self.state = GameState::Menu(MenuState::new(has_save_data()));
                self.ui.menu.selected = 0;
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
}

pub struct RpgGame {
    inner: Rc<RefCell<GameInner>>,
    render_state: Rc<RefCell<RenderState>>,
    _timer: Timer,
}

impl Default for RpgGame {
    fn default() -> Self {
        Self::new()
    }
}

impl RpgGame {
    fn tick(inner: &Rc<RefCell<GameInner>>, render_state: &Rc<RefCell<RenderState>>) {
        let mut inner = inner.borrow_mut();
        inner.update();
        let rs = build_render_state(&inner.state, inner.session.as_ref(), &inner.ui, &inner.data);
        *render_state.borrow_mut() = rs;
        drop(inner);
        repaint(0, 0, 0, 240, 320);
    }

    pub fn new() -> Self {
        let inner = Rc::new(RefCell::new(GameInner {
            state: GameState::Loading(0),
            data: Rc::new(GameData::default()),
            session: None,
            ui: UiState::default(),
        }));

        let render_state = Rc::new(RefCell::new(RenderState::Loading { step: 0 }));

        let timer_inner = Rc::clone(&inner);
        let timer_render_state = Rc::clone(&render_state);
        let timer = Timer::periodic(Duration::from_millis(33), move || {
            Self::tick(&timer_inner, &timer_render_state);
        });

        Self {
            inner,
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
        self.inner.borrow_mut().dispatch(AppAction::KeyDown(key));
    }

    fn on_keyup(&mut self, key: KeyCode) {
        self.inner.borrow_mut().dispatch(AppAction::KeyUp(key));
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}

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
                GameState::Menu => {
                    if let Some(intent) = MenuIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyMenuIntent(intent));
                    }
                }
                GameState::Explore => {
                    for intent in ExploreIntent::intent_for_key(&self.ui.explore, key) {
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
                GameState::Dialog => {
                    if let Some(intent) = DialogIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyDialogIntent(intent));
                    }
                }
                GameState::Shop => {
                    if let Some(intent) = ShopIntent::intent_for_key(key) {
                        effects.push(AppEffect::ApplyShopIntent(intent));
                    }
                }
                GameState::PauseMenu => {
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

    fn apply_update_loading(&mut self) {
        let load_result = if let GameState::Loading(step) = self.state {
            game::lifecycle::load_step(&mut self.data, step)
        } else {
            Ok(false)
        };
        match game::lifecycle::reduce_loading(&self.state, load_result, has_save_data()) {
            game::LoadingEvent::None => {}
            game::LoadingEvent::Advance(step) => self.state = GameState::Loading(step),
            game::LoadingEvent::Loaded(menu_state) => {
                self.state = GameState::Menu;
                self.ui.menu.set_menu(menu_state);
            }
            game::LoadingEvent::Error(msg) => self.state = GameState::Error(msg),
        }
    }

    fn apply_update_movement(&mut self) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let map = self.data.find_map(&s.player.current_map_id);
        let event = game::movement::reduce_tick(
            &self.state,
            &s.movement,
            &s.player,
            map,
            &s.combat,
            &self.data.npcs,
        );
        let moved = game::movement::apply_tick(&mut s.movement, &mut s.player, event);
        if moved {
            game::explore::check_tile_events(&mut s.player, &mut s.combat, &self.data);
        }
    }

    fn apply_update_combat(&mut self) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event =
            game::combat::reduce_tick(&self.state, &s.player, &s.skill_cooldowns, s.mp_regen_timer);
        game::combat::apply_tick(
            &mut self.state,
            &mut s.player,
            &mut s.skill_cooldowns,
            &mut s.mp_regen_timer,
            &mut s.combat,
            &self.data,
            event,
        );
    }

    fn apply_menu_intent(&mut self, intent: MenuIntent) {
        let event = game::menu::reduce(&self.state, &self.ui.menu, intent);
        match event {
            MenuEvent::None => {}
            MenuEvent::SetSelected(selected) => self.ui.menu.set_selected(selected),
            MenuEvent::Action(action) => match action {
                MenuAction::NewGame => {
                    let (state, session, dialog_state) =
                        game::lifecycle::start_new_game(&self.data);
                    self.state = state;
                    self.session = Some(session);
                    self.ui = UiState::default();
                    self.ui.dialog.set(dialog_state);
                }
                MenuAction::Continue => {
                    let (state, session, dialog_state) = game::lifecycle::continue_game(&self.data);
                    self.state = state;
                    self.session = Some(session);
                    self.ui = UiState::default();
                    self.ui.dialog.set(dialog_state);
                }
                MenuAction::Exit => wipi::kernel::exit(0),
            },
        }
    }

    fn apply_explore_intent(&mut self, intent: ExploreIntent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::explore::reduce(&self.state, &s.player, &self.data, intent);
        match event {
            game::ExploreEvent::None => {}
            game::ExploreEvent::MoveDirection(key) => {
                game::movement::on_direction_pressed(&mut s.movement, key);
            }
            game::ExploreEvent::TryNpcInteract { facing } => {
                if let Some(npc_event) = game::npc::reduce(
                    &mut s.player,
                    &self.data,
                    game::NpcIntent::Interact { facing },
                ) {
                    match npc_event {
                        game::NpcEvent::OpenDialog(dialog_state) => {
                            self.ui.dialog.open(dialog_state);
                            self.state = GameState::Dialog;
                        }
                        game::NpcEvent::OpenShop(shop_state) => {
                            self.ui.shop.open(shop_state);
                            self.state = GameState::Shop;
                        }
                    }
                }
            }
            game::ExploreEvent::UseAction(action) => {
                if let Some((slot, skill)) = action.skill() {
                    game::combat::use_skill_action(
                        &mut s.player,
                        &mut s.skill_cooldowns,
                        &mut s.combat,
                        &self.data,
                        slot,
                        skill,
                    );
                } else if let game::CombatEvent::Attack(Some(reward)) = game::combat::reduce(
                    &mut s.combat,
                    game::CombatIntent::PlayerAttack {
                        player_x: s.player.x,
                        player_y: s.player.y,
                        player_atk: s.player.total_atk(),
                        facing: s.player.facing,
                    },
                ) {
                    game::reward::apply_kill_reward(&mut s.player, &self.data, &reward);
                }
            }
            game::ExploreEvent::EnterPauseMenu => {
                self.ui.pause_menu.reset();
                self.state = GameState::PauseMenu;
            }
            game::ExploreEvent::EnterMenu => {
                let _ = game::save_game(&s.player);
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
                self.state = GameState::Menu;
            }
        }
    }

    fn apply_inventory_intent(&mut self, intent: InventoryIntent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::inventory::reduce(
            &self.state,
            &self.ui.inventory,
            s.player.inventory.len(),
            intent,
        );
        match event {
            game::InventoryEvent::None => {}
            game::InventoryEvent::SetSelected(selected) => self.ui.inventory.set_selected(selected),
            game::InventoryEvent::UseSelected(index) => {
                let _ = game::player::reduce(&mut s.player, game::PlayerIntent::UseItem { index });
            }
            game::InventoryEvent::CloseToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_dialog_intent(&mut self, intent: DialogIntent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::dialog::reduce(
            &self.state,
            self.ui.dialog.state.as_ref(),
            &self.data,
            intent,
        );
        match event {
            game::DialogEvent::None => {}
            game::DialogEvent::Transition(transition) => match transition {
                game::DialogTransition::Set(dialog_state) => {
                    self.ui.dialog.open(dialog_state);
                    self.state = GameState::Dialog;
                }
                game::DialogTransition::CloseToExplore => {
                    self.ui.dialog.close();
                    self.state = GameState::Explore;
                }
            },
            game::DialogEvent::Action(action, transition) => {
                if let Some(shop_state) =
                    game::dialog::apply_action(&mut s.player, &self.data, &action)
                {
                    self.ui.shop.open(shop_state);
                    self.state = GameState::Shop;
                    return;
                }

                match transition {
                    game::DialogTransition::Set(dialog_state) => {
                        self.ui.dialog.open(dialog_state);
                        self.state = GameState::Dialog;
                    }
                    game::DialogTransition::CloseToExplore => {
                        self.ui.dialog.close();
                        self.state = GameState::Explore;
                    }
                }
            }
        }
    }

    fn apply_shop_intent(&mut self, intent: ShopIntent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::shop::reduce(&self.state, &s.player, &self.ui.shop, intent);
        match event {
            game::ShopEvent::None => {}
            game::ShopEvent::ErrorNoActiveShop => {
                self.state = GameState::Error(String::from("No active shop state"));
            }
            game::ShopEvent::SetMode(mode) => {
                self.ui.shop.set_mode(mode);
            }
            game::ShopEvent::SetSelected(selected) => self.ui.shop.set_selected(selected),
            game::ShopEvent::BuyItem(item) => {
                let _ =
                    game::player::reduce(&mut s.player, game::PlayerIntent::AddGold(-item.price));
                let _ = game::player::reduce(&mut s.player, game::PlayerIntent::AddItem(item));
            }
            game::ShopEvent::SellSelected(index) => {
                let event =
                    game::player::reduce(&mut s.player, game::PlayerIntent::RemoveItemAt(index));
                if let game::PlayerEvent::ItemRemoved(Some(item)) = event {
                    let _ = game::player::reduce(
                        &mut s.player,
                        game::PlayerIntent::AddGold(item.price / 2),
                    );
                    let inv_len = s.player.inventory.len();
                    if self.ui.shop.selected >= inv_len && self.ui.shop.selected > 0 {
                        self.ui.shop.set_selected(self.ui.shop.selected - 1);
                    }
                }
            }
            game::ShopEvent::CloseToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_pause_menu_intent(&mut self, intent: PauseMenuIntent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::menu::reduce_pause(&self.state, &self.ui.pause_menu, intent);
        match event {
            game::PauseMenuEvent::None => {}
            game::PauseMenuEvent::SetSelected(selected) => {
                self.ui.pause_menu.set_selected(selected)
            }
            game::PauseMenuEvent::OpenInventory => {
                self.ui.inventory.reset();
                self.state = GameState::Inventory;
            }
            game::PauseMenuEvent::OpenStats => self.state = GameState::Stats,
            game::PauseMenuEvent::OpenQuestLog => self.state = GameState::QuestLog,
            game::PauseMenuEvent::SaveAndReturnExplore => {
                let _ = game::save_game(&s.player);
                self.ui.shop.reset();
                self.state = GameState::Explore;
            }
            game::PauseMenuEvent::BackToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_release_movement_key(&mut self, key: KeyCode) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        game::movement::on_key_released(&mut s.movement, key);
    }

    fn apply_effect(&mut self, effect: AppEffect) {
        match effect {
            AppEffect::UpdateLoading => self.apply_update_loading(),
            AppEffect::UpdateMovement => self.apply_update_movement(),
            AppEffect::UpdateCombat => self.apply_update_combat(),
            AppEffect::ApplyMenuIntent(intent) => self.apply_menu_intent(intent),
            AppEffect::ApplyExploreIntent(intent) => self.apply_explore_intent(intent),
            AppEffect::ApplyInventoryIntent(intent) => self.apply_inventory_intent(intent),
            AppEffect::ApplyDialogIntent(intent) => self.apply_dialog_intent(intent),
            AppEffect::ApplyShopIntent(intent) => self.apply_shop_intent(intent),
            AppEffect::ApplyPauseMenuIntent(intent) => self.apply_pause_menu_intent(intent),
            AppEffect::ReturnToExplore => self.state = GameState::Explore,
            AppEffect::ReturnToMenuFromGameOver => {
                self.state = GameState::Menu;
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            AppEffect::ReleaseMovementKey(key) => self.apply_release_movement_key(key),
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

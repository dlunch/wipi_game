#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use alloc::string::String;
use alloc::vec::Vec;

use wipi::app::App;
use wipi::event::KeyCode;
use wipi::framebuffer::Framebuffer;
use wipi::graphics::repaint;
use wipi::wipi_main;

use crate::game::{
    AppAction, AppEffect, CombatState, DialogIntent, GameData, GameState,
    InventoryIntent, InventoryState, MenuIntent, MenuState, MovementState, PauseMenuIntent,
    PlayerState, ShopIntent, explore_intents_for_key, has_save_data, render,
};

pub struct RpgGame {
    state: GameState,
    player: PlayerState,
    data: GameData,
    inventory_state: InventoryState,
    combat: CombatState,
    movement: MovementState,
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
            player: PlayerState::new(String::from("Hero"), "village"),
            data: GameData::default(),
            inventory_state: InventoryState::default(),
            combat: CombatState::default(),
            movement: MovementState::default(),
        }
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
                    for intent in explore_intents_for_key(key) {
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
        let Self {
            state,
            player,
            data,
            inventory_state,
            combat,
            movement,
        } = self;

        match effect {
            AppEffect::UpdateLoading => game::update::update_loading(state, data),
            AppEffect::UpdateMovement => {
                game::update::update_movement(state, movement, player, combat, data);
            }
            AppEffect::UpdateCombat => {
                game::update::update_combat(state, player, combat, data);
            }
            AppEffect::ApplyMenuIntent(intent) => {
                game::handler::handle_menu_input(state, player, combat, data, intent);
            }
            AppEffect::ApplyExploreIntent(intent) => {
                game::handler::handle_explore_input(state, movement, player, combat, data, intent);
            }
            AppEffect::ApplyInventoryIntent(intent) => {
                game::handler::handle_inventory_input(state, player, inventory_state, intent);
            }
            AppEffect::ApplyDialogIntent(intent) => {
                game::handler::handle_dialog_input(state, player, data, intent);
            }
            AppEffect::ApplyShopIntent(intent) => {
                game::handler::handle_shop_input(state, player, intent);
            }
            AppEffect::ApplyPauseMenuIntent(intent) => {
                game::handler::handle_pause_menu_input(state, player, inventory_state, intent);
            }
            AppEffect::ReturnToExplore => *state = GameState::Explore,
            AppEffect::ReturnToMenuFromGameOver => {
                *state = GameState::Menu(MenuState {
                    selected: 0,
                    has_save: has_save_data(),
                });
            }
            AppEffect::ReleaseMovementKey(key) => {
                game::movement::on_key_released(movement, key);
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
        render(
            &self.state,
            &self.player,
            &self.combat,
            &self.data,
            &self.inventory_state,
            fb,
        );
    }
}

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

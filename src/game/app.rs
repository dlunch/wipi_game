mod handler;
mod intent;
mod update;

pub use intent::{AppAction, AppEffect, ExploreIntent};

use alloc::string::String;
use alloc::vec::Vec;

use wipi::event::KeyCode;
use wipi::framebuffer::Framebuffer;

use crate::game::{
    self, CombatState, GameData, GameState, InventoryState, MenuState, MovementState, PlayerState,
    has_save_data, pause_menu_intent_for_key, render,
};

use self::intent::explore_intents_for_key;

pub struct RpgGame {
    pub(crate) state: GameState,
    pub(crate) player: PlayerState,
    pub(crate) data: GameData,
    pub(crate) inventory_state: InventoryState,
    pub(crate) combat: CombatState,
    pub(crate) movement: MovementState,
    pub(crate) mp_regen_timer: u32,
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
            mp_regen_timer: 0,
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
                    if let Some(intent) = MenuState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyMenuIntent(intent));
                    }
                }
                GameState::Explore => {
                    for intent in explore_intents_for_key(key) {
                        effects.push(AppEffect::ApplyExploreIntent(intent));
                    }
                }
                GameState::Inventory => {
                    if let Some(intent) = InventoryState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyInventoryIntent(intent));
                    }
                }
                GameState::Stats | GameState::QuestLog => {
                    if matches!(key, KeyCode::Back | KeyCode::Ok) {
                        effects.push(AppEffect::ReturnToExplore);
                    }
                }
                GameState::Dialog(_) => {
                    if let Some(intent) = game::DialogState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyDialogIntent(intent));
                    }
                }
                GameState::Shop(_) => {
                    if let Some(intent) = game::ShopState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyShopIntent(intent));
                    }
                }
                GameState::PauseMenu(_) => {
                    if let Some(intent) = pause_menu_intent_for_key(key) {
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
            mp_regen_timer,
        } = self;

        match effect {
            AppEffect::UpdateLoading => update::update_loading(state, data),
            AppEffect::UpdateMovement => {
                update::update_movement(state, movement, player, combat, data);
            }
            AppEffect::UpdateCombat => {
                update::update_combat(state, player, combat, data, mp_regen_timer);
            }
            AppEffect::ApplyMenuIntent(intent) => {
                handler::handle_menu_input(state, player, combat, data, intent);
            }
            AppEffect::ApplyExploreIntent(intent) => {
                handler::handle_explore_input(state, movement, player, combat, data, intent);
            }
            AppEffect::ApplyInventoryIntent(intent) => {
                handler::handle_inventory_input(state, player, inventory_state, intent);
            }
            AppEffect::ApplyDialogIntent(intent) => {
                handler::handle_dialog_input(state, player, data, intent);
            }
            AppEffect::ApplyShopIntent(intent) => handler::handle_shop_input(state, player, intent),
            AppEffect::ApplyPauseMenuIntent(intent) => {
                handler::handle_pause_menu_input(state, player, inventory_state, intent);
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

    pub fn dispatch(&mut self, action: AppAction) {
        let effects = self.collect_effects(action);
        for effect in effects {
            self.apply_effect(effect);
        }
    }

    pub fn render(&self, fb: &mut Framebuffer) {
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

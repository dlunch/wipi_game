use alloc::string::String;

use anyhow::Result;

use crate::game::{
    CombatState, GameData, GameEvent, GameState, MovementState, PlayerState, SessionEvent, UiState,
};

#[derive(Clone)]
pub struct SessionState {
    pub player: PlayerState,
    pub combat: CombatState,
    pub movement: MovementState,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
}

impl SessionState {
    pub fn empty() -> Self {
        Self {
            player: PlayerState::new(String::new(), ""),
            combat: CombatState::default(),
            movement: MovementState::default(),
            skill_cooldowns: [0; 3],
            mp_regen_timer: 0,
        }
    }

    pub fn spawn_current_map_enemies(&mut self, data: &GameData) {
        if let Some(map) = data.find_map(&self.player.current_map_id) {
            self.combat.spawn_for_map(map, &data.enemies);
        }
    }

    pub fn apply_event(
        &mut self,
        data: &GameData,
        state: &mut GameState,
        ui: &mut UiState,
        event: &GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Session(session_event) => match session_event {
                SessionEvent::Create => {}
                SessionEvent::SetSkillCooldowns(cooldowns) => {
                    self.skill_cooldowns = *cooldowns;
                }
                SessionEvent::SetMpRegenTimer(timer) => {
                    self.mp_regen_timer = *timer;
                }
                SessionEvent::ResetMovement => {
                    self.movement = MovementState::default();
                }
                SessionEvent::ResetCombat => {
                    self.combat = CombatState::default();
                }
                SessionEvent::SpawnCurrentMapEnemies => {
                    self.spawn_current_map_enemies(data);
                }
                _ => {}
            },
            GameEvent::RestoreSessionStats => self.player.restore_stats(),
            GameEvent::PauseMenu(crate::game::PauseMenuEvent::SaveAndReturnExplore)
            | GameEvent::OpenMenuFromExplore => {
                let _ = crate::game::save_game(&self.player);
            }
            GameEvent::OpenShopById(shop_id) => {
                let _ = open_shop_by_id(data, state, ui, shop_id);
            }
            _ => {}
        }

        self.player.apply_event(data, event)?;
        self.movement
            .apply_event(data, state, &mut self.player, event)?;
        self.combat.apply_event(
            data,
            state,
            &mut self.player,
            &mut self.skill_cooldowns,
            &mut self.mp_regen_timer,
            event,
        )?;
        Ok(())
    }
}

fn open_shop_by_id(
    data: &GameData,
    state: &mut GameState,
    ui: &mut UiState,
    shop_id: &str,
) -> bool {
    let Some(shop) = data.find_shop(shop_id).cloned() else {
        return false;
    };
    let shop_items = data.get_shop_items(&shop);
    ui.shop.open(crate::game::ShopState::new(shop, shop_items));
    if state.can_transition_to(&GameState::Shop) {
        *state = GameState::Shop;
    } else {
        state.set_error(alloc::format!(
            "Invalid state transition: {:?} -> {:?}",
            state,
            GameState::Shop
        ));
    }
    true
}

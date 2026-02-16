use alloc::format;
use alloc::string::String;

use anyhow::Result;

use super::combat::KillReward;

use crate::game::{
    CombatRuntimeEvent, CombatState, GameData, GameEvent, GameState, MovementState, PlayerAction,
    PlayerEvent, PlayerState, SessionEvent, TransitionEvent,
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
        event: &GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Transition(TransitionEvent::MapChanged) => {
                self.spawn_current_map_enemies(data);
            }
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
            GameEvent::Combat(combat_event) => match combat_event {
                CombatRuntimeEvent::SetSkillCooldowns(next_skill_cooldowns) => {
                    self.skill_cooldowns = *next_skill_cooldowns;
                }
                CombatRuntimeEvent::SetMpRegenTimer(next_mp_regen_timer) => {
                    self.mp_regen_timer = *next_mp_regen_timer;
                }
                CombatRuntimeEvent::RecoverMp(recover_mp) => {
                    if *recover_mp > 0 {
                        self.player.stats.recover_mp(*recover_mp);
                    } else if *recover_mp < 0 {
                        self.player.stats.current_mp =
                            (self.player.stats.current_mp + *recover_mp).max(0);
                    }
                }
                CombatRuntimeEvent::Heal(heal) => {
                    if *heal > 0 {
                        let _ = self.player.apply(PlayerAction::Heal(*heal));
                    }
                }
                CombatRuntimeEvent::GrantKillReward {
                    enemy_id,
                    exp,
                    gold,
                } => {
                    self.player.apply_kill_reward(&KillReward {
                        enemy_id: enemy_id.clone(),
                        exp: *exp,
                        gold: *gold,
                    });
                    self.player.apply_quest_kill(data, enemy_id);
                }
                CombatRuntimeEvent::TakeDamage(damage_taken) => {
                    if *damage_taken > 0
                        && matches!(
                            self.player.apply(PlayerAction::TakeDamage(*damage_taken)),
                            PlayerEvent::Died
                        )
                    {
                        if state.can_transition_to(&GameState::GameOver) {
                            *state = GameState::GameOver;
                        } else {
                            state.set_error(format!(
                                "Invalid state transition: {:?} -> {:?}",
                                state,
                                GameState::GameOver
                            ));
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

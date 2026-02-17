use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::QuestProgress;

use crate::game::{
    CharacterState, CombatEvent, CombatState, GameData, GameEvent, GameEventKind,
    GameEventSubscriber, GameState, MovementState, SessionEvent,
};

#[derive(Clone)]
pub struct SessionState {
    pub leader: CharacterState,
    pub companions: Vec<CharacterState>,
    pub quests: Vec<QuestProgress>,
    pub opened_treasures: Vec<(String, usize, usize)>,
    pub combat: CombatState,
    pub movement: MovementState,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
}

impl SessionState {
    pub fn empty() -> Self {
        Self {
            leader: CharacterState::new(String::new(), ""),
            companions: Vec::new(),
            quests: Vec::new(),
            opened_treasures: Vec::new(),
            combat: CombatState::default(),
            movement: MovementState::default(),
            skill_cooldowns: [0; 3],
            mp_regen_timer: 0,
        }
    }

    pub fn has_quest(&self, quest_id: &str) -> bool {
        self.quests
            .iter()
            .any(|q| q.quest_id == quest_id && !q.rewarded)
    }

    pub fn is_quest_complete(&self, quest_id: &str) -> bool {
        self.quests
            .iter()
            .any(|q| q.quest_id == quest_id && q.completed)
    }

    pub fn is_treasure_opened(&self, map_id: &str, x: usize, y: usize) -> bool {
        self.opened_treasures
            .iter()
            .any(|(m, tx, ty)| m == map_id && *tx == x && *ty == y)
    }

    pub fn apply_event(
        &mut self,
        _data: &GameData,
        _state: &mut GameState,
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
                SessionEvent::SpawnCurrentMapEnemies => {}
                SessionEvent::AddQuestProgress(progress) => {
                    if let Some(existing) = self
                        .quests
                        .iter_mut()
                        .find(|quest| quest.quest_id == progress.quest_id)
                    {
                        *existing = progress.clone();
                    } else {
                        self.quests.push(progress.clone());
                    }
                }
                SessionEvent::AddOpenedTreasure { map_id, x, y } => {
                    if !self.is_treasure_opened(map_id, *x, *y) {
                        self.opened_treasures.push((map_id.clone(), *x, *y));
                    }
                }
                _ => {}
            },
            GameEvent::Combat(combat_event) => match combat_event {
                CombatEvent::SetSkillCooldowns(next_skill_cooldowns) => {
                    self.skill_cooldowns = *next_skill_cooldowns;
                }
                CombatEvent::SetMpRegenTimer(next_mp_regen_timer) => {
                    self.mp_regen_timer = *next_mp_regen_timer;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

impl GameEventSubscriber for SessionState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(kind, GameEventKind::Session | GameEventKind::Combat)
    }
}

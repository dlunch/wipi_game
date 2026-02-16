use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::{QuestProgress, QuestType};

use super::combat::KillReward;

use crate::game::{
    CharacterState, CombatEvent, CombatState, GameData, GameEvent, GameState, MovementState,
    PlayerAction, PlayerEvent, SessionEvent,
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

    pub fn apply_tile_event(&mut self, data: &GameData, event: crate::game::TileEvent) {
        match event {
            crate::game::TileEvent::Treasure => {
                let map_id = self.leader.current_map_id.clone();
                if self.is_treasure_opened(&map_id, self.leader.x, self.leader.y) {
                    return;
                }

                if let Some(item_id) = data.newgame.treasure_item.as_deref()
                    && let Some(item) = data.find_item(item_id).cloned()
                {
                    let _ = self.leader.apply(PlayerAction::AddItem(item));
                }
                self.opened_treasures
                    .push((map_id, self.leader.x, self.leader.y));
            }
            crate::game::TileEvent::MapExit(target)
            | crate::game::TileEvent::DungeonEntrance(target) => {
                if target.is_empty() {
                    return;
                }

                let Some(map) = data.find_map(&target) else {
                    return;
                };
                let (x, y) = map
                    .find_player_start()
                    .unwrap_or((self.leader.x, self.leader.y));
                self.leader.current_map_id = map.id.clone();
                self.leader.x = x;
                self.leader.y = y;
            }
        }
    }

    fn apply_quest_kill(&mut self, data: &GameData, enemy_id: &str) {
        let mut updates = Vec::new();
        for progress in &self.quests {
            if progress.completed || progress.rewarded {
                continue;
            }

            if let Some(quest) = data.find_quest(&progress.quest_id)
                && quest.quest_type == QuestType::Kill
                && quest.target_id == enemy_id
            {
                updates.push((progress.quest_id.clone(), quest.target_count));
            }
        }

        for (quest_id, target_count) in updates {
            if let Some(progress) = self.quests.iter_mut().find(|q| q.quest_id == quest_id) {
                progress.current_count = (progress.current_count + 1).min(target_count);
                if progress.current_count >= target_count {
                    progress.completed = true;
                }
            }
        }
    }

    pub fn apply_event(
        &mut self,
        data: &GameData,
        state: &mut GameState,
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
                    if !self.quests.iter().any(|q| q.quest_id == progress.quest_id) {
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
            GameEvent::RestoreHpMp => self.leader.restore_stats(),
            GameEvent::ApplyDialogAction(action) => match action {
                crate::data::DialogAction::GiveQuest(id) => {
                    if !self.quests.iter().any(|q| q.quest_id == *id) {
                        self.quests.push(QuestProgress {
                            quest_id: id.clone(),
                            current_count: 0,
                            completed: false,
                            rewarded: false,
                        });
                    }
                }
                crate::data::DialogAction::CompleteQuest(id) => {
                    let can_reward = self
                        .quests
                        .iter()
                        .any(|q| q.quest_id == *id && q.completed && !q.rewarded);
                    if can_reward && let Some(quest) = data.find_quest(id) {
                        self.leader.stats.add_exp(quest.reward_exp);
                        let _ = self.leader.apply(PlayerAction::AddGold(quest.reward_gold));

                        if let Some(item_id) = &quest.reward_item
                            && let Some(item) = data.find_item(item_id).cloned()
                        {
                            let _ = self.leader.apply(PlayerAction::AddItem(item));
                        }

                        if let Some(progress) = self.quests.iter_mut().find(|q| q.quest_id == *id) {
                            progress.rewarded = true;
                        }
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
                CombatEvent::RecoverMp(recover_mp) => {
                    if *recover_mp > 0 {
                        self.leader.stats.recover_mp(*recover_mp);
                    } else if *recover_mp < 0 {
                        self.leader.stats.current_mp =
                            (self.leader.stats.current_mp + *recover_mp).max(0);
                    }
                }
                CombatEvent::Heal(heal) => {
                    if *heal > 0 {
                        let _ = self.leader.apply(PlayerAction::Heal(*heal));
                    }
                }
                CombatEvent::GrantKillReward {
                    enemy_id,
                    exp,
                    gold,
                } => {
                    self.leader.apply_kill_reward(&KillReward {
                        enemy_id: enemy_id.clone(),
                        exp: *exp,
                        gold: *gold,
                    });
                    self.apply_quest_kill(data, enemy_id);
                }
                CombatEvent::TakeDamage(damage_taken) => {
                    if *damage_taken > 0
                        && matches!(
                            self.leader.apply(PlayerAction::TakeDamage(*damage_taken)),
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
            GameEvent::Movement(crate::game::MovementEvent::Tick(_, maybe_tile_event)) => {
                if let Some(tile_event) = maybe_tile_event.clone() {
                    self.apply_tile_event(data, tile_event);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

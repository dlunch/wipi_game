use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::state::EntityId;
use crate::game::{CombatEvent, GameEvent, GameEventKind, GameEventSubscriber};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatStatsSnapshot {
    pub max_hp: i32,
    pub current_hp: i32,
    pub max_mp: i32,
    pub current_mp: i32,
    pub atk: i32,
    pub def: i32,
}

impl Default for CombatStatsSnapshot {
    fn default() -> Self {
        Self {
            max_hp: 80,
            current_hp: 80,
            max_mp: 30,
            current_mp: 30,
            atk: 12,
            def: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedKind {
    Poison,
    Stun,
    ArmorBreak,
    AttackCooldown,
    SkillCooldown(u8),
    MpRegenTick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedEffect {
    pub kind: TimedKind,
    pub time_left: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimedState {
    pub effects: Vec<TimedEffect>,
}

impl TimedState {
    pub fn time_left(&self, kind: TimedKind) -> u32 {
        self.effects
            .iter()
            .find_map(|effect| (effect.kind == kind).then_some(effect.time_left))
            .unwrap_or(0)
    }

    pub fn set(&mut self, kind: TimedKind, time_left: u32) {
        if let Some(effect) = self.effects.iter_mut().find(|effect| effect.kind == kind) {
            effect.time_left = time_left;
        } else {
            self.effects.push(TimedEffect { kind, time_left });
        }
        self.effects.retain(|effect| effect.time_left > 0);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombatantState {
    pub stats: CombatStatsSnapshot,
    pub timed: TimedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllyCombatantState {
    pub entity_id: EntityId,
    pub combatant: CombatantState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemyCombatantState {
    pub entity_id: EntityId,
    pub source_enemy_id: String,
    pub combatant: CombatantState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombatState {
    pub active: bool,
    pub allies: Vec<AllyCombatantState>,
    pub enemies: Vec<EnemyCombatantState>,
    pub update_counter: u32,
    pub respawn_timer: u32,
}

impl CombatState {
    pub fn combatant(&self, entity_id: EntityId) -> Option<&CombatantState> {
        self.allies
            .iter()
            .find_map(|ally| (ally.entity_id == entity_id).then_some(&ally.combatant))
            .or_else(|| {
                self.enemies
                    .iter()
                    .find_map(|enemy| (enemy.entity_id == entity_id).then_some(&enemy.combatant))
            })
    }

    pub fn combatant_mut(&mut self, entity_id: EntityId) -> Option<&mut CombatantState> {
        if let Some(ally) = self
            .allies
            .iter_mut()
            .find(|ally| ally.entity_id == entity_id)
        {
            return Some(&mut ally.combatant);
        }
        self.enemies
            .iter_mut()
            .find_map(|enemy| (enemy.entity_id == entity_id).then_some(&mut enemy.combatant))
    }

    pub fn apply_event(&mut self, event: &GameEvent) -> Result<()> {
        let GameEvent::Combat(event) = event else {
            return Ok(());
        };
        match event {
            CombatEvent::SetActive(active) => {
                self.active = *active;
            }
            CombatEvent::ClearEnemies => {
                self.enemies.clear();
            }
            CombatEvent::RemoveEnemy(entity_id) => {
                self.enemies.retain(|enemy| enemy.entity_id != *entity_id);
            }
            CombatEvent::SetCombatantMaxHp { entity_id, max_hp } => {
                if let Some(combatant) = self.combatant_mut(*entity_id) {
                    combatant.stats.max_hp = *max_hp;
                }
            }
            CombatEvent::SetCombatantCurrentHp {
                entity_id,
                current_hp,
            } => {
                if let Some(combatant) = self.combatant_mut(*entity_id) {
                    combatant.stats.current_hp = *current_hp;
                }
            }
            CombatEvent::SetCombatantMaxMp { entity_id, max_mp } => {
                if let Some(combatant) = self.combatant_mut(*entity_id) {
                    combatant.stats.max_mp = *max_mp;
                }
            }
            CombatEvent::SetCombatantCurrentMp {
                entity_id,
                current_mp,
            } => {
                if let Some(combatant) = self.combatant_mut(*entity_id) {
                    combatant.stats.current_mp = *current_mp;
                }
            }
            CombatEvent::SetCombatantAtk { entity_id, atk } => {
                if let Some(combatant) = self.combatant_mut(*entity_id) {
                    combatant.stats.atk = *atk;
                }
            }
            CombatEvent::SetCombatantDef { entity_id, def } => {
                if let Some(combatant) = self.combatant_mut(*entity_id) {
                    combatant.stats.def = *def;
                }
            }
            CombatEvent::SetCombatantTimed {
                entity_id,
                kind,
                time_left,
            } => {
                if let Some(combatant) = self.combatant_mut(*entity_id) {
                    combatant.timed.set(*kind, *time_left);
                }
            }
            CombatEvent::SetUpdateCounter(update_counter) => {
                self.update_counter = *update_counter;
            }
            CombatEvent::SetRespawnTimer(respawn_timer) => {
                self.respawn_timer = *respawn_timer;
            }
            CombatEvent::MoveEnemy { .. }
            | CombatEvent::GrantKillReward { .. }
            | CombatEvent::RecoverMp { .. }
            | CombatEvent::Heal { .. }
            | CombatEvent::TakeDamage { .. } => {}
        }
        Ok(())
    }
}

impl GameEventSubscriber for CombatState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(kind, GameEventKind::Combat)
    }
}

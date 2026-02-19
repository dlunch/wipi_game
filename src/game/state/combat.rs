use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::game_event::{CombatEvent, GameEvent, GameEventKind, GameEventSubscriber};
use crate::game::state::EntityId;

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
    pub end_tick: u32,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TimedState {
    pub effects: Vec<TimedEffect>,
}

impl TimedState {
    pub fn end_tick(&self, kind: TimedKind) -> u32 {
        for effect in &self.effects {
            if effect.kind == kind {
                return effect.end_tick;
            }
        }
        0
    }

    pub fn time_left(&self, kind: TimedKind, current_tick: u32) -> u32 {
        let end_tick = self.end_tick(kind);
        if end_tick <= current_tick {
            return 0;
        }
        end_tick - current_tick
    }

    pub fn is_active(&self, kind: TimedKind, current_tick: u32) -> bool {
        self.end_tick(kind) > current_tick
    }

    pub fn set(&mut self, kind: TimedKind, end_tick: u32) {
        if let Some(effect) = self.effects.iter_mut().find(|effect| effect.kind == kind) {
            effect.end_tick = end_tick;
        } else {
            self.effects.push(TimedEffect { kind, end_tick });
        }
        self.effects.retain(|effect| effect.end_tick > 0);
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CombatantState {
    pub timed: TimedState,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AllyCombatantState {
    pub entity_id: EntityId,
    pub combatant: CombatantState,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EnemyCombatantState {
    pub entity_id: EntityId,
    pub source_enemy_id: String,
    pub combatant: CombatantState,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CombatState {
    pub active: bool,
    pub allies: Vec<AllyCombatantState>,
    pub enemies: Vec<EnemyCombatantState>,
    pub respawn_timer: u32,
}

impl CombatState {
    pub fn reset(&mut self) {
        self.active = false;
        self.allies.clear();
        self.enemies.clear();
        self.respawn_timer = 0;
    }

    pub fn combatant(&self, entity_id: EntityId) -> Result<&CombatantState> {
        self.allies
            .iter()
            .find_map(|ally| (ally.entity_id == entity_id).then_some(&ally.combatant))
            .or_else(|| {
                self.enemies
                    .iter()
                    .find_map(|enemy| (enemy.entity_id == entity_id).then_some(&enemy.combatant))
            })
            .ok_or_else(|| anyhow!("Combatant not found: {}", entity_id))
    }

    pub fn combatant_mut(&mut self, entity_id: EntityId) -> Result<&mut CombatantState> {
        if let Some(ally) = self
            .allies
            .iter_mut()
            .find(|ally| ally.entity_id == entity_id)
        {
            return Ok(&mut ally.combatant);
        }
        self.enemies
            .iter_mut()
            .find_map(|enemy| (enemy.entity_id == entity_id).then_some(&mut enemy.combatant))
            .ok_or_else(|| anyhow!("Combatant not found: {}", entity_id))
    }

    pub fn has_combatant(&self, entity_id: EntityId) -> bool {
        self.allies.iter().any(|ally| ally.entity_id == entity_id)
            || self
                .enemies
                .iter()
                .any(|enemy| enemy.entity_id == entity_id)
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
            CombatEvent::SetCombatantTimed {
                entity_id,
                kind,
                end_tick,
            } => {
                let combatant = self.combatant_mut(*entity_id)?;
                combatant.timed.set(*kind, *end_tick);
            }
            CombatEvent::SetRespawnTimer(respawn_timer) => {
                self.respawn_timer = *respawn_timer;
            }
            CombatEvent::MoveEnemy { .. } | CombatEvent::GrantKillReward { .. } => {}
        }
        Ok(())
    }
}

impl GameEventSubscriber for CombatState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(kind, GameEventKind::Combat)
    }
}

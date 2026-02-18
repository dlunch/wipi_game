use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

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
    pub stats: CombatStatsSnapshot,
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
                let combatant = self.combatant_mut(*entity_id)?;
                combatant.stats.max_hp = *max_hp;
                combatant.stats.current_hp = combatant.stats.current_hp.min(*max_hp).max(0);
            }
            CombatEvent::ChangeCombatantHp { entity_id, delta } => {
                let combatant = self.combatant_mut(*entity_id)?;
                let next_hp = combatant.stats.current_hp + *delta;
                combatant.stats.current_hp = next_hp.min(combatant.stats.max_hp).max(0);
            }
            CombatEvent::SetCombatantMaxMp { entity_id, max_mp } => {
                let combatant = self.combatant_mut(*entity_id)?;
                combatant.stats.max_mp = *max_mp;
                combatant.stats.current_mp = combatant.stats.current_mp.min(*max_mp).max(0);
            }
            CombatEvent::ChangeCombatantMp { entity_id, delta } => {
                let combatant = self.combatant_mut(*entity_id)?;
                let next_mp = combatant.stats.current_mp + *delta;
                combatant.stats.current_mp = next_mp.min(combatant.stats.max_mp).max(0);
            }
            CombatEvent::SetCombatantAtk { entity_id, atk } => {
                let combatant = self.combatant_mut(*entity_id)?;
                combatant.stats.atk = *atk;
            }
            CombatEvent::SetCombatantDef { entity_id, def } => {
                let combatant = self.combatant_mut(*entity_id)?;
                combatant.stats.def = *def;
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

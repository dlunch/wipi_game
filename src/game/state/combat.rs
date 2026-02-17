use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::{Enemy, SkillType};
use crate::game::{CombatEvent, GameEvent, GameEventKind, GameEventSubscriber};

#[derive(Debug, Clone)]
pub struct KillReward {
    pub enemy_id: String,
    pub exp: i32,
    pub gold: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct SkillEffect {
    pub x: usize,
    pub y: usize,
    pub effect_type: SkillType,
    pub timer: u32,
}

#[derive(Debug, Clone)]
pub struct FieldEnemy {
    pub instance_id: u32,
    pub data: Enemy,
    pub x: usize,
    pub y: usize,
    pub hp: i32,
    pub attack_cooldown: u32,
}

impl FieldEnemy {
    pub fn new(data: Enemy, x: usize, y: usize, instance_id: u32) -> Self {
        let hp = data.hp;
        Self {
            instance_id,
            data,
            x,
            y,
            hp,
            attack_cooldown: 0,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CombatState {
    pub enemies: Vec<FieldEnemy>,
    pub player_attack_cooldown: u32,
    pub skill_effects: Vec<SkillEffect>,
    pub update_counter: u32,
    pub respawn_timer: u32,
    pub respawn_positions: Vec<(usize, usize, usize)>,
    pub next_enemy_instance_id: u32,
}

impl CombatState {
    pub fn apply_event(&mut self, event: &GameEvent) -> Result<()> {
        let GameEvent::Combat(event) = event else {
            return Ok(());
        };
        match event {
            CombatEvent::SetUpdateCounter(update_counter) => {
                self.update_counter = *update_counter;
            }
            CombatEvent::SetMapEnemies {
                enemies,
                respawn_positions,
                next_enemy_instance_id,
            } => {
                self.enemies = enemies.clone();
                self.respawn_positions = respawn_positions.clone();
                self.respawn_timer = 0;
                self.player_attack_cooldown = 0;
                self.skill_effects.clear();
                self.update_counter = 0;
                self.next_enemy_instance_id = (*next_enemy_instance_id).max(1);
            }
            CombatEvent::EnemySpawn(enemy) => {
                self.enemies.push(enemy.clone());
            }
            CombatEvent::EnemyDespawn(enemy_id) => {
                self.enemies.retain(|enemy| enemy.instance_id != *enemy_id);
            }
            CombatEvent::EnemyMove { enemy_id, x, y } => {
                if let Some(enemy) = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.instance_id == *enemy_id)
                {
                    enemy.x = *x;
                    enemy.y = *y;
                }
            }
            CombatEvent::EnemyHpSet { enemy_id, hp } => {
                if let Some(enemy) = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.instance_id == *enemy_id)
                {
                    enemy.hp = *hp;
                }
            }
            CombatEvent::EnemyAttackCooldownSet { enemy_id, cooldown } => {
                if let Some(enemy) = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.instance_id == *enemy_id)
                {
                    enemy.attack_cooldown = *cooldown;
                }
            }
            CombatEvent::EnemyHitFlashSet { .. }
            | CombatEvent::SetPlayerHitFlash(_)
            | CombatEvent::SetSkillCooldowns(_)
            | CombatEvent::RecoverMp(_)
            | CombatEvent::Heal(_)
            | CombatEvent::GrantKillReward { .. }
            | CombatEvent::TakeDamage(_) => {}
            CombatEvent::SetPlayerAttackCooldown(cooldown) => {
                self.player_attack_cooldown = *cooldown;
            }
            CombatEvent::TickSkillEffects => {
                self.skill_effects.retain_mut(|effect| {
                    if effect.timer == 0 {
                        return false;
                    }
                    effect.timer -= 1;
                    effect.timer > 0
                });
            }
            CombatEvent::SetSkillEffects(skill_effects) => {
                self.skill_effects = skill_effects.clone();
            }
            CombatEvent::SetRespawnTimer(respawn_timer) => {
                self.respawn_timer = *respawn_timer;
            }
            CombatEvent::SetNextEnemyInstanceId(next_enemy_instance_id) => {
                self.next_enemy_instance_id = *next_enemy_instance_id;
            }
        }
        Ok(())
    }
}

impl GameEventSubscriber for CombatState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(kind, GameEventKind::Combat)
    }
}

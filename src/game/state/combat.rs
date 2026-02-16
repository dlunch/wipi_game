use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::{Enemy, Map, SkillType};
use crate::game::{CombatEvent, GameEvent};

const HIT_FLASH_DURATION: u32 = 10;
const ENEMY_ATTACK_COOLDOWN: u32 = 30;

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
    pub hit_flash: u32,
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
            hit_flash: 0,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.hp = (self.hp - damage).max(0);
        self.hit_flash = HIT_FLASH_DURATION;
    }

    pub fn distance_to(&self, px: usize, py: usize) -> usize {
        self.x.abs_diff(px) + self.y.abs_diff(py)
    }

    pub fn update(&mut self, player_x: usize, player_y: usize, map: &Map) {
        if self.hit_flash > 0 {
            self.hit_flash -= 1;
        }
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }

        if self.distance_to(player_x, player_y) > 1 {
            self.move_towards(player_x, player_y, map);
        }
    }

    fn move_towards(&mut self, target_x: usize, target_y: usize, map: &Map) {
        let dx: i32 = match target_x.cmp(&self.x) {
            core::cmp::Ordering::Greater => 1,
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
        };
        let dy: i32 = match target_y.cmp(&self.y) {
            core::cmp::Ordering::Greater => 1,
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
        };

        let new_x = self.x.checked_add_signed(dx as isize);
        let new_y = self.y.checked_add_signed(dy as isize);

        if let Some(nx) = new_x
            && dx != 0
            && map.get_tile(nx, self.y).is_passable()
        {
            self.x = nx;
            return;
        }
        if let Some(ny) = new_y
            && dy != 0
            && map.get_tile(self.x, ny).is_passable()
        {
            self.y = ny;
        }
    }

    pub fn can_attack(&self) -> bool {
        self.attack_cooldown == 0
    }

    pub fn do_attack(&mut self) -> i32 {
        self.attack_cooldown = ENEMY_ATTACK_COOLDOWN;
        self.data.atk
    }
}

#[derive(Debug, Default, Clone)]
pub struct CombatState {
    pub enemies: Vec<FieldEnemy>,
    pub player_attack_cooldown: u32,
    pub player_hit_flash: u32,
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
            CombatEvent::SetMapEnemies {
                enemies,
                respawn_positions,
                next_enemy_instance_id,
            } => {
                self.enemies = enemies.clone();
                self.respawn_positions = respawn_positions.clone();
                self.respawn_timer = 0;
                self.player_attack_cooldown = 0;
                self.player_hit_flash = 0;
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
            CombatEvent::EnemyHitFlashSet {
                enemy_id,
                hit_flash,
            } => {
                if let Some(enemy) = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.instance_id == *enemy_id)
                {
                    enemy.hit_flash = *hit_flash;
                }
            }
            CombatEvent::SetPlayerAttackCooldown(cooldown) => {
                self.player_attack_cooldown = *cooldown;
            }
            CombatEvent::SetPlayerHitFlash(hit_flash) => {
                self.player_hit_flash = *hit_flash;
            }
            CombatEvent::SetSkillEffects(skill_effects) => {
                self.skill_effects = skill_effects.clone();
            }
            CombatEvent::SetUpdateCounter(update_counter) => {
                self.update_counter = *update_counter;
            }
            CombatEvent::SetRespawnTimer(respawn_timer) => {
                self.respawn_timer = *respawn_timer;
            }
            CombatEvent::SetNextEnemyInstanceId(next_enemy_instance_id) => {
                self.next_enemy_instance_id = *next_enemy_instance_id;
            }
            CombatEvent::SetSkillCooldowns(_)
            | CombatEvent::SetMpRegenTimer(_)
            | CombatEvent::RecoverMp(_)
            | CombatEvent::Heal(_)
            | CombatEvent::GrantKillReward { .. }
            | CombatEvent::TakeDamage(_) => {}
        }
        Ok(())
    }
}

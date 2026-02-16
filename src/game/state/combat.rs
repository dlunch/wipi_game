use alloc::string::String;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::{Direction, Enemy, Map, Skill, SkillType, Tile};
use crate::game::{CombatRuntimeEvent, GameEvent};

const HIT_FLASH_DURATION: u32 = 10;
const ENEMY_ATTACK_COOLDOWN: u32 = 30;
const PLAYER_ATTACK_COOLDOWN: u32 = 15;
const ATTACK_EFFECT_DURATION: u32 = 6;
const SKILL_EFFECT_DURATION: u32 = 8;
const HEAL_EFFECT_DURATION: u32 = 15;

#[derive(Debug)]
pub enum CombatAction<'a> {
    PlayerAttack {
        player_x: usize,
        player_y: usize,
        player_atk: i32,
        facing: Direction,
    },
    UseSkill {
        skill: &'a Skill,
        player_x: usize,
        player_y: usize,
        player_atk: i32,
        facing: Direction,
    },
}

#[derive(Debug)]
pub enum CombatEvent {
    Attack(Option<KillReward>),
    Skill(SkillResult),
}

#[derive(Debug, Clone)]
pub struct SkillResult {
    pub player_effects: Vec<PlayerEffect>,
    pub kills: Vec<KillReward>,
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerEffect {
    Heal(i32),
}

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
    pub fn apply(&mut self, action: CombatAction<'_>) -> CombatEvent {
        match action {
            CombatAction::PlayerAttack {
                player_x,
                player_y,
                player_atk,
                facing,
            } => CombatEvent::Attack(self.player_attack(player_x, player_y, player_atk, facing)),
            CombatAction::UseSkill {
                skill,
                player_x,
                player_y,
                player_atk,
                facing,
            } => CombatEvent::Skill(self.use_skill(skill, player_x, player_y, player_atk, facing)),
        }
    }

    pub fn spawn_for_map(&mut self, map: &Map, enemy_data: &[Enemy]) {
        self.enemies.clear();
        self.respawn_positions.clear();
        self.respawn_timer = 0;
        self.player_attack_cooldown = 0;
        self.player_hit_flash = 0;
        self.skill_effects.clear();
        self.update_counter = 0;
        self.next_enemy_instance_id = 1;

        let mut enemy_tiles: Vec<(usize, usize)> = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get_tile(x, y) == Tile::Enemy {
                    enemy_tiles.push((x, y));
                }
            }
        }

        if enemy_tiles.is_empty() || map.encounters.is_empty() {
            return;
        }

        let available_enemies: Vec<&Enemy> = map
            .encounters
            .iter()
            .filter_map(|(id, _)| enemy_data.iter().find(|e| &e.id == id))
            .collect();

        if available_enemies.is_empty() {
            return;
        }

        for (i, (x, y)) in enemy_tiles.iter().enumerate() {
            let enemy_idx = i % available_enemies.len();
            let enemy = available_enemies[enemy_idx];
            let instance_id = self.allocate_enemy_instance_id();
            self.enemies
                .push(FieldEnemy::new(enemy.clone(), *x, *y, instance_id));
            self.respawn_positions.push((*x, *y, enemy_idx));
        }
    }

    fn allocate_enemy_instance_id(&mut self) -> u32 {
        let id = self.next_enemy_instance_id;
        self.next_enemy_instance_id = self.next_enemy_instance_id.wrapping_add(1);
        if self.next_enemy_instance_id == 0 {
            self.next_enemy_instance_id = 1;
        }
        id.max(1)
    }

    fn player_attack(
        &mut self,
        player_x: usize,
        player_y: usize,
        player_atk: i32,
        facing: Direction,
    ) -> Option<KillReward> {
        if self.player_attack_cooldown > 0 {
            return None;
        }

        let (tx, ty) = facing.apply(player_x, player_y);

        self.skill_effects.push(SkillEffect {
            x: tx,
            y: ty,
            effect_type: SkillType::Attack,
            timer: ATTACK_EFFECT_DURATION,
        });

        for enemy in &mut self.enemies {
            if enemy.x == tx && enemy.y == ty && !enemy.is_dead() {
                let damage = (player_atk - enemy.data.def / 2).max(1);
                enemy.take_damage(damage);
                self.player_attack_cooldown = PLAYER_ATTACK_COOLDOWN;

                return if enemy.is_dead() {
                    Some(KillReward {
                        enemy_id: enemy.data.id.clone(),
                        exp: enemy.data.exp,
                        gold: enemy.data.gold,
                    })
                } else {
                    None
                };
            }
        }

        self.player_attack_cooldown = PLAYER_ATTACK_COOLDOWN;
        None
    }

    fn use_skill(
        &mut self,
        skill: &Skill,
        player_x: usize,
        player_y: usize,
        player_atk: i32,
        facing: Direction,
    ) -> SkillResult {
        let mut player_effects = Vec::new();
        let mut kills = Vec::new();
        let damage = skill.power + player_atk / 2;

        match skill.skill_type {
            SkillType::Attack => {}
            SkillType::Ranged => {
                for dist in 1..=skill.range {
                    let (tx, ty) = facing.apply_distance(player_x, player_y, dist);
                    self.skill_effects.push(SkillEffect {
                        x: tx,
                        y: ty,
                        effect_type: SkillType::Ranged,
                        timer: SKILL_EFFECT_DURATION,
                    });
                    if let Some(kill) = self.damage_enemy_at(tx, ty, damage) {
                        kills.push(kill);
                        break;
                    }
                }
            }
            SkillType::Area => {
                for dir in [
                    Direction::Up,
                    Direction::Down,
                    Direction::Left,
                    Direction::Right,
                ] {
                    let (tx, ty) = dir.apply(player_x, player_y);
                    self.skill_effects.push(SkillEffect {
                        x: tx,
                        y: ty,
                        effect_type: SkillType::Area,
                        timer: SKILL_EFFECT_DURATION,
                    });
                    if let Some(kill) = self.damage_enemy_at(tx, ty, damage) {
                        kills.push(kill);
                    }
                }
            }
            SkillType::Heal => {}
        }

        if skill.heal_power > 0 {
            self.skill_effects.push(SkillEffect {
                x: player_x,
                y: player_y,
                effect_type: SkillType::Heal,
                timer: HEAL_EFFECT_DURATION,
            });
            player_effects.push(PlayerEffect::Heal(skill.heal_power));
        }

        SkillResult {
            player_effects,
            kills,
        }
    }

    fn damage_enemy_at(&mut self, x: usize, y: usize, damage: i32) -> Option<KillReward> {
        for enemy in &mut self.enemies {
            if enemy.x == x && enemy.y == y && !enemy.is_dead() {
                let actual_damage = (damage - enemy.data.def / 2).max(1);
                enemy.take_damage(actual_damage);

                if enemy.is_dead() {
                    return Some(KillReward {
                        enemy_id: enemy.data.id.clone(),
                        exp: enemy.data.exp,
                        gold: enemy.data.gold,
                    });
                }
                return None;
            }
        }
        None
    }
}

impl CombatState {
    pub fn apply_event(&mut self, event: &GameEvent) -> Result<()> {
        let GameEvent::Combat(event) = event else {
            return Ok(());
        };
        match event {
            CombatRuntimeEvent::EnemySpawn(enemy) => {
                self.enemies.push(enemy.clone());
            }
            CombatRuntimeEvent::EnemyDespawn(enemy_id) => {
                self.enemies.retain(|enemy| enemy.instance_id != *enemy_id);
            }
            CombatRuntimeEvent::EnemyMove { enemy_id, x, y } => {
                if let Some(enemy) = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.instance_id == *enemy_id)
                {
                    enemy.x = *x;
                    enemy.y = *y;
                }
            }
            CombatRuntimeEvent::EnemyHpSet { enemy_id, hp } => {
                if let Some(enemy) = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.instance_id == *enemy_id)
                {
                    enemy.hp = *hp;
                }
            }
            CombatRuntimeEvent::EnemyAttackCooldownSet { enemy_id, cooldown } => {
                if let Some(enemy) = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.instance_id == *enemy_id)
                {
                    enemy.attack_cooldown = *cooldown;
                }
            }
            CombatRuntimeEvent::EnemyHitFlashSet {
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
            CombatRuntimeEvent::SetPlayerAttackCooldown(cooldown) => {
                self.player_attack_cooldown = *cooldown;
            }
            CombatRuntimeEvent::SetPlayerHitFlash(hit_flash) => {
                self.player_hit_flash = *hit_flash;
            }
            CombatRuntimeEvent::SetSkillEffects(skill_effects) => {
                self.skill_effects = skill_effects.clone();
            }
            CombatRuntimeEvent::SetUpdateCounter(update_counter) => {
                self.update_counter = *update_counter;
            }
            CombatRuntimeEvent::SetRespawnTimer(respawn_timer) => {
                self.respawn_timer = *respawn_timer;
            }
            CombatRuntimeEvent::SetNextEnemyInstanceId(next_enemy_instance_id) => {
                self.next_enemy_instance_id = *next_enemy_instance_id;
            }
            CombatRuntimeEvent::SetSkillCooldowns(_)
            | CombatRuntimeEvent::SetMpRegenTimer(_)
            | CombatRuntimeEvent::RecoverMp(_)
            | CombatRuntimeEvent::Heal(_)
            | CombatRuntimeEvent::GrantKillReward { .. }
            | CombatRuntimeEvent::TakeDamage(_) => {}
        }
        Ok(())
    }
}

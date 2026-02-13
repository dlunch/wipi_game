use alloc::vec::Vec;

use crate::data::{Enemy, Map, Skill, SkillType, Tile};

const HIT_FLASH_DURATION: u32 = 10;
const ENEMY_ATTACK_COOLDOWN: u32 = 30;
const ENEMY_MOVE_INTERVAL: u32 = 8;
const PLAYER_ATTACK_COOLDOWN: u32 = 15;
const ATTACK_EFFECT_DURATION: u32 = 6;
const SKILL_EFFECT_DURATION: u32 = 8;
const HEAL_EFFECT_DURATION: u32 = 15;

pub enum CombatIntent<'a> {
    SpawnEnemies {
        map: &'a Map,
        enemy_data: &'a [Enemy],
    },
    Tick {
        player_x: usize,
        player_y: usize,
        player_def: i32,
        map: &'a Map,
        enemy_data: &'a [Enemy],
    },
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

pub enum CombatEvent {
    None,
    Tick(CombatResult),
    Attack(Option<KillReward>),
    Skill(SkillResult),
}

#[derive(Debug, Clone)]
pub struct FieldEnemy {
    pub data: Enemy,
    pub x: usize,
    pub y: usize,
    pub hp: i32,
    pub attack_cooldown: u32,
    pub hit_flash: u32,
}

impl FieldEnemy {
    pub fn new(data: Enemy, x: usize, y: usize) -> Self {
        let hp = data.hp;
        Self {
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

#[derive(Debug, Clone, Copy)]
pub struct SkillEffect {
    pub x: usize,
    pub y: usize,
    pub effect_type: SkillType,
    pub timer: u32,
}

#[derive(Default)]
pub struct CombatState {
    pub enemies: Vec<FieldEnemy>,
    pub player_attack_cooldown: u32,
    pub player_hit_flash: u32,
    pub skill_effects: Vec<SkillEffect>,
    pub update_counter: u32,
    pub respawn_timer: u32,
    pub respawn_positions: Vec<(usize, usize, usize)>,
}

pub fn reduce(state: &mut CombatState, intent: CombatIntent<'_>) -> CombatEvent {
    match intent {
        CombatIntent::SpawnEnemies { map, enemy_data } => {
            spawn_enemies(state, map, enemy_data);
            CombatEvent::None
        }
        CombatIntent::Tick {
            player_x,
            player_y,
            player_def,
            map,
            enemy_data,
        } => CombatEvent::Tick(update(
            state, player_x, player_y, player_def, map, enemy_data,
        )),
        CombatIntent::PlayerAttack {
            player_x,
            player_y,
            player_atk,
            facing,
        } => CombatEvent::Attack(player_attack(state, player_x, player_y, player_atk, facing)),
        CombatIntent::UseSkill {
            skill,
            player_x,
            player_y,
            player_atk,
            facing,
        } => CombatEvent::Skill(use_skill(
            state, skill, player_x, player_y, player_atk, facing,
        )),
    }
}

pub fn spawn_enemies(state: &mut CombatState, map: &Map, enemy_data: &[Enemy]) {
    state.enemies.clear();
    state.respawn_positions.clear();
    state.respawn_timer = 0;
    state.player_attack_cooldown = 0;
    state.player_hit_flash = 0;
    state.skill_effects.clear();
    state.update_counter = 0;

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
        state.enemies.push(FieldEnemy::new(enemy.clone(), *x, *y));
        state.respawn_positions.push((*x, *y, enemy_idx));
    }
}

fn update(
    state: &mut CombatState,
    player_x: usize,
    player_y: usize,
    player_def: i32,
    map: &Map,
    enemy_data: &[Enemy],
) -> CombatResult {
    state.update_counter = state.update_counter.wrapping_add(1);

    if state.player_attack_cooldown > 0 {
        state.player_attack_cooldown -= 1;
    }
    if state.player_hit_flash > 0 {
        state.player_hit_flash -= 1;
    }

    for effect in &mut state.skill_effects {
        if effect.timer > 0 {
            effect.timer -= 1;
        }
    }
    state.skill_effects.retain(|e| e.timer > 0);

    let mut damage_taken = 0;

    if state.update_counter.is_multiple_of(ENEMY_MOVE_INTERVAL) {
        for enemy in &mut state.enemies {
            if !enemy.is_dead() {
                enemy.update(player_x, player_y, map);
            }
        }
    }

    for enemy in &mut state.enemies {
        if enemy.is_dead() {
            continue;
        }

        if enemy.distance_to(player_x, player_y) <= 1 && enemy.can_attack() {
            let raw_damage = enemy.do_attack();
            let actual_damage = (raw_damage - player_def / 2).max(1);
            damage_taken += actual_damage;
            state.player_hit_flash = HIT_FLASH_DURATION;
        }
    }

    state.enemies.retain(|e| !e.is_dead());

    try_respawn(state, player_x, player_y, map, enemy_data);

    CombatResult { damage_taken }
}

fn try_respawn(
    state: &mut CombatState,
    player_x: usize,
    player_y: usize,
    map: &Map,
    enemy_data: &[Enemy],
) {
    const RESPAWN_DELAY: u32 = 300;
    const RESPAWN_DISTANCE: usize = 8;

    if state.respawn_positions.is_empty() {
        return;
    }

    let max_enemies = state.respawn_positions.len();
    if state.enemies.len() >= max_enemies {
        state.respawn_timer = 0;
        return;
    }

    state.respawn_timer += 1;
    if state.respawn_timer < RESPAWN_DELAY {
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

    for (x, y, enemy_idx) in &state.respawn_positions {
        let distance = x.abs_diff(player_x) + y.abs_diff(player_y);
        if distance < RESPAWN_DISTANCE {
            continue;
        }

        let already_exists = state.enemies.iter().any(|e| e.x == *x && e.y == *y);
        if already_exists {
            continue;
        }

        if let Some(enemy) = available_enemies.get(*enemy_idx) {
            state
                .enemies
                .push(FieldEnemy::new((*enemy).clone(), *x, *y));
            state.respawn_timer = 0;
            return;
        }
    }
}

pub fn player_attack(
    state: &mut CombatState,
    player_x: usize,
    player_y: usize,
    player_atk: i32,
    facing: Direction,
) -> Option<KillReward> {
    if state.player_attack_cooldown > 0 {
        return None;
    }

    let (tx, ty) = facing.apply(player_x, player_y);

    state.skill_effects.push(SkillEffect {
        x: tx,
        y: ty,
        effect_type: SkillType::Attack,
        timer: ATTACK_EFFECT_DURATION,
    });

    for enemy in &mut state.enemies {
        if enemy.x == tx && enemy.y == ty && !enemy.is_dead() {
            let damage = (player_atk - enemy.data.def / 2).max(1);
            enemy.take_damage(damage);
            state.player_attack_cooldown = PLAYER_ATTACK_COOLDOWN;

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

    state.player_attack_cooldown = PLAYER_ATTACK_COOLDOWN;
    None
}

pub fn enemy_at(state: &CombatState, x: usize, y: usize) -> bool {
    state
        .enemies
        .iter()
        .any(|e| e.x == x && e.y == y && !e.is_dead())
}

pub fn use_skill(
    state: &mut CombatState,
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
                state.skill_effects.push(SkillEffect {
                    x: tx,
                    y: ty,
                    effect_type: SkillType::Ranged,
                    timer: SKILL_EFFECT_DURATION,
                });
                if let Some(kill) = damage_enemy_at(state, tx, ty, damage) {
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
                state.skill_effects.push(SkillEffect {
                    x: tx,
                    y: ty,
                    effect_type: SkillType::Area,
                    timer: SKILL_EFFECT_DURATION,
                });
                if let Some(kill) = damage_enemy_at(state, tx, ty, damage) {
                    kills.push(kill);
                }
            }
        }
        SkillType::Heal => {}
    }

    if skill.heal_power > 0 {
        state.skill_effects.push(SkillEffect {
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

fn damage_enemy_at(state: &mut CombatState, x: usize, y: usize, damage: i32) -> Option<KillReward> {
    for enemy in &mut state.enemies {
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

pub struct SkillResult {
    pub player_effects: Vec<PlayerEffect>,
    pub kills: Vec<KillReward>,
}

pub enum PlayerEffect {
    Heal(i32),
}

pub struct CombatResult {
    pub damage_taken: i32,
}

pub struct KillReward {
    pub enemy_id: alloc::string::String,
    pub exp: i32,
    pub gold: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn apply(&self, x: usize, y: usize) -> (usize, usize) {
        self.apply_distance(x, y, 1)
    }

    pub fn apply_distance(&self, x: usize, y: usize, dist: usize) -> (usize, usize) {
        match self {
            Direction::Up => (x, y.saturating_sub(dist)),
            Direction::Down => (x, y.saturating_add(dist)),
            Direction::Left => (x.saturating_sub(dist), y),
            Direction::Right => (x.saturating_add(dist), y),
        }
    }
}

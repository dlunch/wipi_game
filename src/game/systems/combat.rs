use alloc::vec::Vec;

use crate::data::{Direction, Enemy, Map, Skill, SkillType, Tile};
use crate::game::PlayerState;

const HIT_FLASH_DURATION: u32 = 10;
const ENEMY_ATTACK_COOLDOWN: u32 = 30;
const ENEMY_MOVE_INTERVAL: u32 = 8;
const PLAYER_ATTACK_COOLDOWN: u32 = 15;
const ATTACK_EFFECT_DURATION: u32 = 6;
const SKILL_EFFECT_DURATION: u32 = 8;
const HEAL_EFFECT_DURATION: u32 = 15;
const MP_REGEN_INTERVAL: u32 = 60;

#[derive(Debug)]
pub enum CombatIntent<'a> {
    Tick {
        player_x: usize,
        player_y: usize,
        player_def: i32,
        skill_cooldowns: [u32; 3],
        mp_regen_timer: u32,
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

#[derive(Debug)]
pub enum CombatEvent {
    Tick(CombatResult),
    Attack(Option<KillReward>),
    Skill(SkillResult),
}

struct TickContext<'a> {
    player_x: usize,
    player_y: usize,
    player_def: i32,
    skill_cooldowns: [u32; 3],
    mp_regen_timer: u32,
    map: &'a Map,
    enemy_data: &'a [Enemy],
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

pub fn apply(state: &mut CombatState, intent: CombatIntent<'_>) -> CombatEvent {
    match intent {
        CombatIntent::Tick {
            player_x,
            player_y,
            player_def,
            skill_cooldowns,
            mp_regen_timer,
            map,
            enemy_data,
        } => CombatEvent::Tick(update(
            state,
            TickContext {
                player_x,
                player_y,
                player_def,
                skill_cooldowns,
                mp_regen_timer,
                map,
                enemy_data,
            },
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

pub fn spawn_for_map(state: &mut CombatState, map: &Map, enemy_data: &[Enemy]) {
    spawn_enemies(state, map, enemy_data);
}

fn spawn_enemies(state: &mut CombatState, map: &Map, enemy_data: &[Enemy]) {
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

fn update(state: &mut CombatState, ctx: TickContext<'_>) -> CombatResult {
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
                enemy.update(ctx.player_x, ctx.player_y, ctx.map);
            }
        }
    }

    for enemy in &mut state.enemies {
        if enemy.is_dead() {
            continue;
        }

        if enemy.distance_to(ctx.player_x, ctx.player_y) <= 1 && enemy.can_attack() {
            let raw_damage = enemy.do_attack();
            let actual_damage = (raw_damage - ctx.player_def / 2).max(1);
            damage_taken += actual_damage;
            state.player_hit_flash = HIT_FLASH_DURATION;
        }
    }

    state.enemies.retain(|e| !e.is_dead());

    try_respawn(state, ctx.player_x, ctx.player_y, ctx.map, ctx.enemy_data);

    let (next_skill_cooldowns, next_mp_regen_timer, recover_mp) =
        tick_resource_state(ctx.skill_cooldowns, ctx.mp_regen_timer);

    CombatResult {
        damage_taken,
        next_skill_cooldowns,
        next_mp_regen_timer,
        recover_mp,
    }
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

fn player_attack(
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

#[cfg(test)]
pub fn enemy_at(state: &CombatState, x: usize, y: usize) -> bool {
    state
        .enemies
        .iter()
        .any(|e| e.x == x && e.y == y && !e.is_dead())
}

fn use_skill(
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

#[derive(Debug, Clone)]
pub struct SkillResult {
    pub player_effects: Vec<PlayerEffect>,
    pub kills: Vec<KillReward>,
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerEffect {
    Heal(i32),
}

#[derive(Debug, Clone, Copy)]
pub struct CombatResult {
    pub damage_taken: i32,
    pub next_skill_cooldowns: [u32; 3],
    pub next_mp_regen_timer: u32,
    pub recover_mp: i32,
}

#[derive(Debug, Clone)]
pub struct KillReward {
    pub enemy_id: alloc::string::String,
    pub exp: i32,
    pub gold: i32,
}

fn tick_resource_state(skill_cooldowns: [u32; 3], mp_regen_timer: u32) -> ([u32; 3], u32, i32) {
    let mut next_skill_cooldowns = skill_cooldowns;
    for cooldown in &mut next_skill_cooldowns {
        if *cooldown > 0 {
            *cooldown -= 1;
        }
    }

    let mut next_mp_regen_timer = mp_regen_timer + 1;
    let mut recover_mp = 0;
    if next_mp_regen_timer >= MP_REGEN_INTERVAL {
        next_mp_regen_timer = 0;
        recover_mp = 1;
    }

    (next_skill_cooldowns, next_mp_regen_timer, recover_mp)
}

pub fn apply_tick(
    player: &mut PlayerState,
    skill_cooldowns: &mut [u32; 3],
    mp_regen_timer: &mut u32,
    event: CombatResult,
) -> i32 {
    *skill_cooldowns = event.next_skill_cooldowns;
    *mp_regen_timer = event.next_mp_regen_timer;
    if event.recover_mp > 0 {
        player.stats.recover_mp(event.recover_mp);
    }

    event.damage_taken
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Direction, Enemy, Map, Tile};
    use crate::game::{GameData, GameState, PlayerState};

    fn make_enemy(id: &str, hp: i32, atk: i32, def: i32, exp: i32, gold: i32) -> Enemy {
        Enemy {
            id: String::from(id),
            name: String::from(id),
            hp,
            atk,
            def,
            exp,
            gold,
        }
    }

    fn make_test_map(
        width: usize,
        height: usize,
        tiles: Vec<Tile>,
        encounters: Vec<(String, i32)>,
    ) -> Map {
        Map {
            id: String::from("test_map"),
            name: String::from("Test Map"),
            width,
            height,
            tiles,
            encounters,
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: false,
        }
    }

    fn make_player() -> PlayerState {
        PlayerState::new(String::from("Hero"), "test_map")
    }

    #[test]
    fn field_enemy_new_sets_initial_values() {
        let enemy = make_enemy("slime", 20, 5, 2, 10, 3);
        let field = FieldEnemy::new(enemy.clone(), 3, 4);

        assert_eq!(field.data.id, enemy.id);
        assert_eq!(field.x, 3);
        assert_eq!(field.y, 4);
        assert_eq!(field.hp, 20);
        assert_eq!(field.attack_cooldown, 0);
        assert_eq!(field.hit_flash, 0);
    }

    #[test]
    fn field_enemy_is_dead_checks_hp_threshold() {
        let mut field = FieldEnemy::new(make_enemy("slime", 1, 5, 0, 1, 1), 0, 0);
        assert!(!field.is_dead());
        field.hp = 0;
        assert!(field.is_dead());
    }

    #[test]
    fn field_enemy_take_damage_clamps_hp_and_sets_flash() {
        let mut field = FieldEnemy::new(make_enemy("slime", 12, 5, 0, 1, 1), 0, 0);

        field.take_damage(5);
        assert_eq!(field.hp, 7);
        assert_eq!(field.hit_flash, HIT_FLASH_DURATION);

        field.take_damage(100);
        assert_eq!(field.hp, 0);
        assert_eq!(field.hit_flash, HIT_FLASH_DURATION);
    }

    #[test]
    fn field_enemy_distance_to_is_manhattan_distance() {
        let field = FieldEnemy::new(make_enemy("slime", 10, 5, 0, 1, 1), 2, 3);
        assert_eq!(field.distance_to(5, 7), 7);
        assert_eq!(field.distance_to(2, 3), 0);
    }

    #[test]
    fn field_enemy_can_attack_and_do_attack_sets_cooldown() {
        let mut field = FieldEnemy::new(make_enemy("slime", 10, 9, 0, 1, 1), 0, 0);
        assert!(field.can_attack());

        let damage = field.do_attack();
        assert_eq!(damage, 9);
        assert_eq!(field.attack_cooldown, ENEMY_ATTACK_COOLDOWN);
        assert!(!field.can_attack());
    }

    #[test]
    fn field_enemy_move_towards_moves_on_passable_tile() {
        let map = make_test_map(
            4,
            4,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
            Vec::new(),
        );
        let mut field = FieldEnemy::new(make_enemy("slime", 10, 5, 0, 1, 1), 0, 0);

        field.move_towards(2, 0, &map);
        assert_eq!(field.x, 1);
        assert_eq!(field.y, 0);
    }

    #[test]
    fn spawn_enemies_places_on_enemy_tiles() {
        let mut state = CombatState::default();
        let map = make_test_map(
            3,
            2,
            vec![
                Tile::Floor,
                Tile::Enemy,
                Tile::Floor,
                Tile::Enemy,
                Tile::Floor,
                Tile::Floor,
            ],
            vec![(String::from("slime"), 1)],
        );
        let enemies = vec![make_enemy("slime", 10, 4, 0, 5, 2)];

        spawn_enemies(&mut state, &map, &enemies);

        assert_eq!(state.enemies.len(), 2);
        assert!(state.enemies.iter().any(|e| e.x == 1 && e.y == 0));
        assert!(state.enemies.iter().any(|e| e.x == 0 && e.y == 1));
    }

    #[test]
    fn spawn_enemies_with_no_enemy_tiles_produces_none() {
        let mut state = CombatState::default();
        let map = make_test_map(
            2,
            2,
            vec![Tile::Floor, Tile::Floor, Tile::Floor, Tile::Floor],
            vec![(String::from("slime"), 1)],
        );
        let enemies = vec![make_enemy("slime", 10, 4, 0, 5, 2)];

        spawn_enemies(&mut state, &map, &enemies);

        assert!(state.enemies.is_empty());
    }

    #[test]
    fn spawn_enemies_with_empty_encounters_produces_none() {
        let mut state = CombatState::default();
        let map = make_test_map(
            2,
            2,
            vec![Tile::Enemy, Tile::Floor, Tile::Floor, Tile::Floor],
            Vec::new(),
        );
        let enemies = vec![make_enemy("slime", 10, 4, 0, 5, 2)];

        spawn_enemies(&mut state, &map, &enemies);

        assert!(state.enemies.is_empty());
    }

    #[test]
    fn reduce_player_attack_hits_enemy_in_facing_direction() {
        let mut state = CombatState::default();
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("slime", 20, 4, 0, 5, 2), 2, 1));

        let event = apply(
            &mut state,
            CombatIntent::PlayerAttack {
                player_x: 1,
                player_y: 1,
                player_atk: 10,
                facing: Direction::Right,
            },
        );

        assert!(matches!(event, CombatEvent::Attack(None)));
        assert_eq!(state.enemies[0].hp, 10);
    }

    #[test]
    fn reduce_player_attack_kill_returns_reward() {
        let mut state = CombatState::default();
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("slime", 5, 4, 0, 11, 7), 2, 1));

        let event = apply(
            &mut state,
            CombatIntent::PlayerAttack {
                player_x: 1,
                player_y: 1,
                player_atk: 10,
                facing: Direction::Right,
            },
        );

        let CombatEvent::Attack(reward) = event else {
            panic!("expected CombatEvent::Attack");
        };
        assert!(reward.is_some());
        if let Some(kill) = reward {
            assert_eq!(kill.enemy_id, "slime");
            assert_eq!(kill.exp, 11);
            assert_eq!(kill.gold, 7);
        }
    }

    #[test]
    fn reduce_player_attack_cooldown_prevents_double_attack() {
        let mut state = CombatState::default();
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("slime", 30, 4, 0, 5, 2), 2, 1));

        let first = apply(
            &mut state,
            CombatIntent::PlayerAttack {
                player_x: 1,
                player_y: 1,
                player_atk: 10,
                facing: Direction::Right,
            },
        );
        let hp_after_first = state.enemies[0].hp;

        let second = apply(
            &mut state,
            CombatIntent::PlayerAttack {
                player_x: 1,
                player_y: 1,
                player_atk: 10,
                facing: Direction::Right,
            },
        );

        assert!(matches!(first, CombatEvent::Attack(None)));
        assert!(matches!(second, CombatEvent::Attack(None)));
        assert_eq!(state.enemies[0].hp, hp_after_first);
        assert_eq!(state.player_attack_cooldown, PLAYER_ATTACK_COOLDOWN);
    }

    #[test]
    fn reduce_player_attack_miss_returns_none() {
        let mut state = CombatState::default();

        let event = apply(
            &mut state,
            CombatIntent::PlayerAttack {
                player_x: 1,
                player_y: 1,
                player_atk: 10,
                facing: Direction::Right,
            },
        );

        assert!(matches!(event, CombatEvent::Attack(None)));
        assert_eq!(state.player_attack_cooldown, PLAYER_ATTACK_COOLDOWN);
    }

    #[test]
    fn reduce_use_skill_ranged_hits_enemy_in_line() {
        let mut state = CombatState::default();
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("slime", 20, 4, 0, 5, 2), 3, 1));

        let event = apply(
            &mut state,
            CombatIntent::UseSkill {
                skill: &crate::data::Skill::FIREBALL,
                player_x: 1,
                player_y: 1,
                player_atk: 10,
                facing: Direction::Right,
            },
        );

        let CombatEvent::Skill(result) = event else {
            panic!("expected CombatEvent::Skill");
        };
        assert_eq!(result.kills.len(), 1);
        assert_eq!(result.kills[0].enemy_id, "slime");
    }

    #[test]
    fn reduce_use_skill_area_hits_four_adjacent_enemies() {
        let mut state = CombatState::default();
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("up", 15, 4, 0, 1, 1), 2, 1));
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("down", 15, 4, 0, 1, 1), 2, 3));
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("left", 15, 4, 0, 1, 1), 1, 2));
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("right", 15, 4, 0, 1, 1), 3, 2));

        let event = apply(
            &mut state,
            CombatIntent::UseSkill {
                skill: &crate::data::Skill::SPIN_ATTACK,
                player_x: 2,
                player_y: 2,
                player_atk: 10,
                facing: Direction::Up,
            },
        );

        let CombatEvent::Skill(result) = event else {
            panic!("expected CombatEvent::Skill");
        };
        assert_eq!(result.kills.len(), 4);
    }

    #[test]
    fn reduce_use_skill_heal_adds_player_heal_effect() {
        let mut state = CombatState::default();

        let event = apply(
            &mut state,
            CombatIntent::UseSkill {
                skill: &crate::data::Skill::HEAL,
                player_x: 4,
                player_y: 5,
                player_atk: 10,
                facing: Direction::Left,
            },
        );

        let CombatEvent::Skill(result) = event else {
            panic!("expected CombatEvent::Skill");
        };
        assert!(matches!(
            result.player_effects.as_slice(),
            [PlayerEffect::Heal(30)]
        ));
    }

    #[test]
    fn enemy_at_checks_live_dead_and_empty_tiles() {
        let mut state = CombatState::default();
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("live", 10, 4, 0, 1, 1), 1, 1));
        state
            .enemies
            .push(FieldEnemy::new(make_enemy("dead", 10, 4, 0, 1, 1), 2, 2));
        state.enemies[1].hp = 0;

        assert!(enemy_at(&state, 1, 1));
        assert!(!enemy_at(&state, 2, 2));
        assert!(!enemy_at(&state, 0, 0));
    }

    #[test]
    fn update_combat_enemy_damage_reduces_player_hp() {
        let mut player = make_player();
        let initial_hp = player.stats.current_hp;
        player.x = 1;
        player.y = 1;
        let mut cooldowns = [0; 3];
        let mut mp_regen_timer = 0;
        let mut combat = CombatState::default();
        combat
            .enemies
            .push(FieldEnemy::new(make_enemy("slime", 20, 20, 0, 1, 1), 2, 1));

        let mut data = GameData::default();
        data.maps.push(make_test_map(
            3,
            3,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
            Vec::new(),
        ));

        let map = data.find_map("test_map").expect("map exists");
        let CombatEvent::Tick(result) = apply(
            &mut combat,
            CombatIntent::Tick {
                player_x: player.x,
                player_y: player.y,
                player_def: player.total_def(),
                skill_cooldowns: cooldowns,
                mp_regen_timer,
                map,
                enemy_data: &data.enemies,
            },
        ) else {
            panic!("expected combat tick event");
        };

        let damage_taken = apply_tick(&mut player, &mut cooldowns, &mut mp_regen_timer, result);
        if damage_taken > 0 {
            let _ = crate::game::player::apply(
                &mut player,
                crate::game::PlayerIntent::TakeDamage(damage_taken),
            );
        }

        assert!(player.stats.current_hp < initial_hp);
    }

    #[test]
    fn update_combat_player_death_sets_game_over() {
        let mut player = make_player();
        player.stats.current_hp = 1;
        player.x = 1;
        player.y = 1;
        let mut cooldowns = [0; 3];
        let mut mp_regen_timer = 0;
        let mut combat = CombatState::default();
        combat
            .enemies
            .push(FieldEnemy::new(make_enemy("slime", 20, 20, 0, 1, 1), 2, 1));

        let mut data = GameData::default();
        data.maps.push(make_test_map(
            3,
            3,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
            Vec::new(),
        ));

        let map = data.find_map("test_map").expect("map exists");
        let CombatEvent::Tick(result) = apply(
            &mut combat,
            CombatIntent::Tick {
                player_x: player.x,
                player_y: player.y,
                player_def: player.total_def(),
                skill_cooldowns: cooldowns,
                mp_regen_timer,
                map,
                enemy_data: &data.enemies,
            },
        ) else {
            panic!("expected combat tick event");
        };

        let damage_taken = apply_tick(&mut player, &mut cooldowns, &mut mp_regen_timer, result);
        let mut game_state = GameState::Explore;
        if damage_taken > 0
            && matches!(
                crate::game::player::apply(
                    &mut player,
                    crate::game::PlayerIntent::TakeDamage(damage_taken)
                ),
                crate::game::PlayerEvent::Died
            )
        {
            game_state = GameState::GameOver;
        }

        assert!(matches!(game_state, GameState::GameOver));
        assert_eq!(player.stats.current_hp, 0);
    }

    #[test]
    fn update_combat_decrements_cooldown_and_regens_mp() {
        let mut player = make_player();
        player.stats.current_mp = 10;
        let mut cooldowns = [2, 0, 1];
        let mut mp_regen_timer = MP_REGEN_INTERVAL - 1;
        let map = make_test_map(
            2,
            2,
            vec![Tile::Floor, Tile::Floor, Tile::Floor, Tile::Floor],
            Vec::new(),
        );
        let CombatEvent::Tick(result) = apply(
            &mut CombatState::default(),
            CombatIntent::Tick {
                player_x: player.x,
                player_y: player.y,
                player_def: player.total_def(),
                skill_cooldowns: cooldowns,
                mp_regen_timer,
                map: &map,
                enemy_data: &[],
            },
        ) else {
            panic!("expected combat tick event");
        };
        let _ = apply_tick(&mut player, &mut cooldowns, &mut mp_regen_timer, result);

        assert_eq!(cooldowns, [1, 0, 0]);
        assert_eq!(mp_regen_timer, 0);
        assert_eq!(player.stats.current_mp, 11);
    }
}

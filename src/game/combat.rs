use alloc::vec::Vec;

use crate::data::{Enemy, Map, Tile};

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
        self.hit_flash = 10;
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
        let dx = match target_x.cmp(&self.x) {
            core::cmp::Ordering::Greater => 1i32,
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
        };
        let dy = match target_y.cmp(&self.y) {
            core::cmp::Ordering::Greater => 1i32,
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
        };

        let new_x = (self.x as i32 + dx) as usize;
        let new_y = (self.y as i32 + dy) as usize;

        if dx != 0 && map.get_tile(new_x, self.y).is_passable() {
            self.x = new_x;
        } else if dy != 0 && map.get_tile(self.x, new_y).is_passable() {
            self.y = new_y;
        }
    }

    pub fn can_attack(&self) -> bool {
        self.attack_cooldown == 0
    }

    pub fn do_attack(&mut self) -> i32 {
        self.attack_cooldown = 30;
        self.data.atk
    }
}

#[derive(Default)]
pub struct CombatSystem {
    pub enemies: Vec<FieldEnemy>,
    pub player_attack_cooldown: u32,
    pub player_hit_flash: u32,
    update_counter: u32,
}

impl CombatSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn_enemies(&mut self, map: &Map, enemy_data: &[Enemy]) {
        self.enemies.clear();

        for (enemy_id, _weight) in &map.encounters {
            if let Some(data) = enemy_data.iter().find(|e| &e.id == enemy_id) {
                for y in 0..map.height {
                    for x in 0..map.width {
                        if map.get_tile(x, y) == Tile::Enemy
                            && !self.enemies.iter().any(|e| e.x == x && e.y == y)
                        {
                            self.enemies.push(FieldEnemy::new(data.clone(), x, y));
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn update(
        &mut self,
        player_x: usize,
        player_y: usize,
        player_def: i32,
        map: &Map,
    ) -> CombatResult {
        self.update_counter = self.update_counter.wrapping_add(1);

        if self.player_attack_cooldown > 0 {
            self.player_attack_cooldown -= 1;
        }
        if self.player_hit_flash > 0 {
            self.player_hit_flash -= 1;
        }

        let mut damage_taken = 0;

        if self.update_counter.is_multiple_of(8) {
            for enemy in &mut self.enemies {
                if !enemy.is_dead() {
                    enemy.update(player_x, player_y, map);
                }
            }
        }

        for enemy in &mut self.enemies {
            if enemy.is_dead() {
                continue;
            }

            if enemy.distance_to(player_x, player_y) <= 1 && enemy.can_attack() {
                let raw_damage = enemy.do_attack();
                let actual_damage = (raw_damage - player_def / 2).max(1);
                damage_taken += actual_damage;
                self.player_hit_flash = 10;
            }
        }

        self.enemies.retain(|e| !e.is_dead());

        CombatResult { damage_taken }
    }

    pub fn player_attack(
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

        for enemy in &mut self.enemies {
            if enemy.x == tx && enemy.y == ty && !enemy.is_dead() {
                let damage = (player_atk - enemy.data.def / 2).max(1);
                enemy.take_damage(damage);
                self.player_attack_cooldown = 15;

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

        self.player_attack_cooldown = 15;
        None
    }

    pub fn enemy_at(&self, x: usize, y: usize) -> bool {
        self.enemies
            .iter()
            .any(|e| e.x == x && e.y == y && !e.is_dead())
    }
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
        match self {
            Direction::Up => (x, y.saturating_sub(1)),
            Direction::Down => (x, y + 1),
            Direction::Left => (x.saturating_sub(1), y),
            Direction::Right => (x + 1, y),
        }
    }
}

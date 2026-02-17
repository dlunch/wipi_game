use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::QuestProgress;
use crate::game::state::FieldEnemy;

use crate::game::{
    CharacterState, CombatEvent, CombatState, GameData, GameEvent, GameEventKind,
    GameEventSubscriber, MovementState, WorldEvent,
};

#[derive(Clone)]
pub struct WorldState {
    pub leader: CharacterState,
    pub companions: Vec<CharacterState>,
    pub quests: Vec<QuestProgress>,
    pub opened_treasures: Vec<(String, usize, usize)>,
    pub combat: CombatState,
    pub movement: MovementState,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
    occupied_map_id: String,
    occupied_width: usize,
    occupied_height: usize,
    npc_occupied_tiles: Vec<bool>,
    enemy_occupied_tiles: Vec<bool>,
    enemy_positions: Vec<(u32, usize, usize)>,
}

impl WorldState {
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
            occupied_map_id: String::new(),
            occupied_width: 0,
            occupied_height: 0,
            npc_occupied_tiles: Vec::new(),
            enemy_occupied_tiles: Vec::new(),
            enemy_positions: Vec::new(),
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

    pub fn is_occupied(&self, x: usize, y: usize) -> bool {
        if self.occupied_width == 0 || self.occupied_height == 0 {
            return false;
        }
        if x >= self.occupied_width || y >= self.occupied_height {
            return true;
        }
        let idx = y * self.occupied_width + x;
        self.npc_occupied_tiles.get(idx).copied().unwrap_or(false)
            || self.enemy_occupied_tiles.get(idx).copied().unwrap_or(false)
    }

    pub fn apply_event(&mut self, data: &GameData, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::World(session_event) => match session_event {
                WorldEvent::Create => {
                    self.clear_occupancy();
                }
                WorldEvent::SetSkillCooldowns(cooldowns) => {
                    self.skill_cooldowns = *cooldowns;
                }
                WorldEvent::SetMpRegenTimer(timer) => {
                    self.mp_regen_timer = *timer;
                }
                WorldEvent::ResetMovement => {
                    self.movement = MovementState::default();
                }
                WorldEvent::ResetCombat => {
                    self.combat = CombatState::default();
                    self.clear_enemy_occupancy();
                }
                WorldEvent::SetPlayerMap(map_id) => {
                    self.rebuild_npc_occupancy_for_map(data, map_id);
                }
                WorldEvent::AddQuestProgress(progress) => {
                    if let Some(existing) = self
                        .quests
                        .iter_mut()
                        .find(|quest| quest.quest_id == progress.quest_id)
                    {
                        *existing = progress.clone();
                    } else {
                        self.quests.push(progress.clone());
                    }
                }
                WorldEvent::AddOpenedTreasure { map_id, x, y } => {
                    if !self.is_treasure_opened(map_id, *x, *y) {
                        self.opened_treasures.push((map_id.clone(), *x, *y));
                    }
                }
                _ => {}
            },
            GameEvent::Combat(combat_event) => match combat_event {
                CombatEvent::SetMapEnemies { enemies, .. } => {
                    self.rebuild_enemy_occupancy_from_list(enemies);
                }
                CombatEvent::EnemySpawn(enemy) => {
                    self.add_enemy(enemy.instance_id, enemy.x, enemy.y, enemy.hp > 0);
                }
                CombatEvent::EnemyDespawn(enemy_id) => {
                    self.remove_enemy(*enemy_id);
                }
                CombatEvent::EnemyMove { enemy_id, x, y } => {
                    self.move_enemy(*enemy_id, *x, *y);
                }
                CombatEvent::EnemyHpSet { enemy_id, hp } => {
                    if *hp <= 0 {
                        self.remove_enemy(*enemy_id);
                    }
                }
                CombatEvent::SetSkillCooldowns(next_skill_cooldowns) => {
                    self.skill_cooldowns = *next_skill_cooldowns;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

impl WorldState {
    fn clear_occupancy(&mut self) {
        self.occupied_map_id.clear();
        self.occupied_width = 0;
        self.occupied_height = 0;
        self.npc_occupied_tiles.clear();
        self.enemy_occupied_tiles.clear();
        self.enemy_positions.clear();
    }

    fn rebuild_npc_occupancy_for_map(&mut self, data: &GameData, map_id: &str) {
        self.occupied_map_id = map_id.into();
        if let Some(map) = data.find_map(map_id) {
            self.occupied_width = map.width;
            self.occupied_height = map.height;
            let len = map.width * map.height;
            self.npc_occupied_tiles = vec![false; len];
            self.enemy_occupied_tiles = vec![false; len];
            self.enemy_positions.clear();

            for (x, y, _) in &map.npcs {
                if *x < map.width && *y < map.height {
                    self.npc_occupied_tiles[*y * map.width + *x] = true;
                }
            }
        } else {
            self.clear_occupancy();
        }
    }

    fn clear_enemy_occupancy(&mut self) {
        for occupied in &mut self.enemy_occupied_tiles {
            *occupied = false;
        }
        self.enemy_positions.clear();
    }

    fn rebuild_enemy_occupancy_from_list(&mut self, enemies: &[FieldEnemy]) {
        self.clear_enemy_occupancy();
        for enemy in enemies {
            self.add_enemy(enemy.instance_id, enemy.x, enemy.y, enemy.hp > 0);
        }
    }

    fn add_enemy(&mut self, enemy_id: u32, x: usize, y: usize, alive: bool) {
        if !alive || x >= self.occupied_width || y >= self.occupied_height {
            return;
        }
        let idx = y * self.occupied_width + x;
        if let Some(tile) = self.enemy_occupied_tiles.get_mut(idx) {
            *tile = true;
        }
        if let Some(pos) = self
            .enemy_positions
            .iter_mut()
            .find(|(id, _, _)| *id == enemy_id)
        {
            pos.1 = x;
            pos.2 = y;
        } else {
            self.enemy_positions.push((enemy_id, x, y));
        }
    }

    fn remove_enemy(&mut self, enemy_id: u32) {
        if let Some(idx) = self
            .enemy_positions
            .iter()
            .position(|(id, _, _)| *id == enemy_id)
        {
            let (_, x, y) = self.enemy_positions.swap_remove(idx);
            if x < self.occupied_width && y < self.occupied_height {
                self.enemy_occupied_tiles[y * self.occupied_width + x] = false;
            }
        }
    }

    fn move_enemy(&mut self, enemy_id: u32, x: usize, y: usize) {
        if x >= self.occupied_width || y >= self.occupied_height {
            self.remove_enemy(enemy_id);
            return;
        }

        if let Some((_, old_x, old_y)) = self
            .enemy_positions
            .iter_mut()
            .find(|(id, _, _)| *id == enemy_id)
        {
            if *old_x < self.occupied_width && *old_y < self.occupied_height {
                self.enemy_occupied_tiles[*old_y * self.occupied_width + *old_x] = false;
            }
            *old_x = x;
            *old_y = y;
        } else {
            self.enemy_positions.push((enemy_id, x, y));
        }
        self.enemy_occupied_tiles[y * self.occupied_width + x] = true;
    }
}

impl GameEventSubscriber for WorldState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(kind, GameEventKind::World | GameEventKind::Combat)
    }
}

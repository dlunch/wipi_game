use crate::data::{Direction, Item};
use crate::game::{
    CombatAction, CombatEvent, CombatState, GameData, MovementState, MovementTickEvent,
    PlayerAction, PlayerEffect, PlayerEvent, PlayerState, RuntimeEvent, TileApplyEvent, TileEvent,
};

pub trait SessionEventApplier {
    fn handles_event(&self, event: &RuntimeEvent) -> bool;
    fn apply_runtime_event(&mut self, event: &RuntimeEvent) -> bool;
}

pub enum DialogActionResult {
    None,
    OpenShop(alloc::string::String),
}

pub struct SessionState {
    pub player: PlayerState,
    pub combat: CombatState,
    pub movement: MovementState,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
}

impl SessionState {
    pub fn on_direction_pressed(&mut self, direction: Direction) {
        self.movement.on_direction_pressed(direction);
    }

    pub fn on_direction_released(&mut self, direction: Direction) {
        self.movement.on_direction_released(direction);
    }

    pub fn restore_stats(&mut self) {
        self.player.stats.current_hp = self.player.stats.max_hp;
        self.player.stats.current_mp = self.player.stats.max_mp;
    }

    pub fn use_inventory_item(&mut self, index: usize) {
        let _ = self.player.apply(PlayerAction::UseItem { index });
    }

    pub fn buy_shop_item(&mut self, item: Item) {
        let _ = self.player.apply(PlayerAction::AddGold(-item.price));
        let _ = self.player.apply(PlayerAction::AddItem(item));
    }

    pub fn sell_inventory_item(&mut self, index: usize) -> Option<Item> {
        let event = self.player.apply(PlayerAction::RemoveItemAt(index));
        if let PlayerEvent::ItemRemoved(Some(item)) = event {
            let _ = self.player.apply(PlayerAction::AddGold(item.price / 2));
            Some(item)
        } else {
            None
        }
    }

    pub fn apply_movement_tick(
        &mut self,
        data: &GameData,
        movement_event: MovementTickEvent,
        tile_event: Option<TileEvent>,
    ) {
        let moved = self.movement.apply_tick(&mut self.player, movement_event);
        if moved && let Some(tile_event) = tile_event {
            let _: TileApplyEvent = self.player.apply_tile_event(data, tile_event);
        }
    }

    pub fn apply_event(&mut self, event: &RuntimeEvent) -> bool {
        match event {
            RuntimeEvent::Combat(event) => match event {
                crate::game::CombatRuntimeEvent::EnemySpawn(enemy) => {
                    self.combat.enemies.push(enemy.clone());
                    false
                }
                crate::game::CombatRuntimeEvent::EnemyDespawn(enemy_id) => {
                    self.combat
                        .enemies
                        .retain(|enemy| enemy.instance_id != *enemy_id);
                    false
                }
                crate::game::CombatRuntimeEvent::EnemyMove { enemy_id, x, y } => {
                    if let Some(enemy) = self
                        .combat
                        .enemies
                        .iter_mut()
                        .find(|enemy| enemy.instance_id == *enemy_id)
                    {
                        enemy.x = *x;
                        enemy.y = *y;
                    }
                    false
                }
                crate::game::CombatRuntimeEvent::EnemyHpSet { enemy_id, hp } => {
                    if let Some(enemy) = self
                        .combat
                        .enemies
                        .iter_mut()
                        .find(|enemy| enemy.instance_id == *enemy_id)
                    {
                        enemy.hp = *hp;
                    }
                    false
                }
                crate::game::CombatRuntimeEvent::EnemyAttackCooldownSet { enemy_id, cooldown } => {
                    if let Some(enemy) = self
                        .combat
                        .enemies
                        .iter_mut()
                        .find(|enemy| enemy.instance_id == *enemy_id)
                    {
                        enemy.attack_cooldown = *cooldown;
                    }
                    false
                }
                crate::game::CombatRuntimeEvent::EnemyHitFlashSet {
                    enemy_id,
                    hit_flash,
                } => {
                    if let Some(enemy) = self
                        .combat
                        .enemies
                        .iter_mut()
                        .find(|enemy| enemy.instance_id == *enemy_id)
                    {
                        enemy.hit_flash = *hit_flash;
                    }
                    false
                }
                crate::game::CombatRuntimeEvent::SetPlayerAttackCooldown(cooldown) => {
                    self.combat.player_attack_cooldown = *cooldown;
                    false
                }
                crate::game::CombatRuntimeEvent::SetPlayerHitFlash(hit_flash) => {
                    self.combat.player_hit_flash = *hit_flash;
                    false
                }
                crate::game::CombatRuntimeEvent::SetSkillEffects(skill_effects) => {
                    self.combat.skill_effects = skill_effects.clone();
                    false
                }
                crate::game::CombatRuntimeEvent::SetUpdateCounter(update_counter) => {
                    self.combat.update_counter = *update_counter;
                    false
                }
                crate::game::CombatRuntimeEvent::SetRespawnTimer(respawn_timer) => {
                    self.combat.respawn_timer = *respawn_timer;
                    false
                }
                crate::game::CombatRuntimeEvent::SetNextEnemyInstanceId(next_enemy_instance_id) => {
                    self.combat.next_enemy_instance_id = *next_enemy_instance_id;
                    false
                }
                crate::game::CombatRuntimeEvent::SetSkillCooldowns(next_skill_cooldowns) => {
                    self.skill_cooldowns = *next_skill_cooldowns;
                    false
                }
                crate::game::CombatRuntimeEvent::SetMpRegenTimer(next_mp_regen_timer) => {
                    self.mp_regen_timer = *next_mp_regen_timer;
                    false
                }
                crate::game::CombatRuntimeEvent::RecoverMp(recover_mp) => {
                    if *recover_mp > 0 {
                        self.player.stats.recover_mp(*recover_mp);
                    }
                    false
                }
                crate::game::CombatRuntimeEvent::TakeDamage(damage_taken) => {
                    *damage_taken > 0
                        && matches!(
                            self.player.apply(PlayerAction::TakeDamage(*damage_taken)),
                            PlayerEvent::Died
                        )
                }
            },
            _ => false,
        }
    }
}

impl SessionEventApplier for SessionState {
    fn handles_event(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Combat(_))
    }

    fn apply_runtime_event(&mut self, event: &RuntimeEvent) -> bool {
        self.apply_event(event)
    }
}

impl SessionState {
    pub fn spawn_current_map_enemies(&mut self, data: &GameData) {
        if let Some(map) = data.find_map(&self.player.current_map_id) {
            self.combat.spawn_for_map(map, &data.enemies);
        }
    }

    pub fn apply_explore_action(&mut self, data: &GameData, action: crate::game::ExploreAction) {
        if let Some((slot, skill)) = action.skill() {
            if !self
                .player
                .can_use_skill(&self.skill_cooldowns, slot, skill.mp_cost)
            {
                return;
            }

            let combat_event = self.combat.apply(CombatAction::UseSkill {
                skill,
                player_x: self.player.x,
                player_y: self.player.y,
                player_atk: self.player.total_atk(),
                facing: self.player.facing,
            });
            let CombatEvent::Skill(result) = combat_event else {
                return;
            };

            self.skill_cooldowns[slot] = skill.cooldown;
            self.player.stats.current_mp = (self.player.stats.current_mp - skill.mp_cost).max(0);

            for effect in &result.player_effects {
                match effect {
                    PlayerEffect::Heal(amount) => {
                        let _ = self.player.apply(PlayerAction::Heal(*amount));
                    }
                }
            }

            self.player.apply_kill_rewards(&result.kills);
            for reward in &result.kills {
                self.player.apply_quest_kill(data, &reward.enemy_id);
            }
            return;
        }

        if let CombatEvent::Attack(Some(reward)) = self.combat.apply(CombatAction::PlayerAttack {
            player_x: self.player.x,
            player_y: self.player.y,
            player_atk: self.player.total_atk(),
            facing: self.player.facing,
        }) {
            self.player.apply_kill_reward(&reward);
            self.player.apply_quest_kill(data, &reward.enemy_id);
        }
    }

    pub fn apply_dialog_action(
        &mut self,
        data: &GameData,
        action: &crate::data::DialogAction,
    ) -> DialogActionResult {
        match action {
            crate::data::DialogAction::GiveQuest(id) => {
                if !self.player.quests.iter().any(|q| q.quest_id == *id) {
                    self.player.quests.push(crate::data::QuestProgress {
                        quest_id: id.clone(),
                        current_count: 0,
                        completed: false,
                        rewarded: false,
                    });
                }
            }
            crate::data::DialogAction::CompleteQuest(id) => {
                let can_reward = self
                    .player
                    .quests
                    .iter()
                    .any(|q| q.quest_id == *id && q.completed && !q.rewarded);
                if can_reward && let Some(quest) = data.find_quest(id) {
                    self.player.stats.add_exp(quest.reward_exp);
                    let _ = self.player.apply(PlayerAction::AddGold(quest.reward_gold));

                    if let Some(item_id) = &quest.reward_item
                        && let Some(item) = data.find_item(item_id).cloned()
                    {
                        let _ = self.player.apply(PlayerAction::AddItem(item));
                    }

                    if let Some(progress) =
                        self.player.quests.iter_mut().find(|q| q.quest_id == *id)
                    {
                        progress.rewarded = true;
                    }
                }
            }
            crate::data::DialogAction::GiveItem(id) => {
                if let Some(item) = data.find_item(id).cloned() {
                    let _ = self.player.apply(PlayerAction::AddItem(item));
                }
            }
            crate::data::DialogAction::TakeItem(id) => {
                if let Some(index) = self.player.inventory.iter().position(|item| item.id == *id) {
                    let _ = self.player.apply(PlayerAction::RemoveItemAt(index));
                }
            }
            crate::data::DialogAction::GiveGold(amount) => {
                let _ = self.player.apply(PlayerAction::AddGold(*amount));
            }
            crate::data::DialogAction::TakeGold(amount) => {
                let _ = self.player.apply(PlayerAction::AddGold(-*amount));
            }
            crate::data::DialogAction::OpenShop(shop_id) => {
                return DialogActionResult::OpenShop(shop_id.clone());
            }
            crate::data::DialogAction::Heal => {
                self.player.stats.current_hp = self.player.stats.max_hp;
                self.player.stats.current_mp = self.player.stats.max_mp;
            }
        }

        DialogActionResult::None
    }
}

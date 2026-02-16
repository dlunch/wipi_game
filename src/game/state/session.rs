use crate::data::{Direction, Item};
use anyhow::{Result, anyhow};

use crate::data::Tile;
use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{
    CombatAction, CombatEvent, CombatState, GameData, MovementState, MovementTickEvent,
    PlayerAction, PlayerEffect, PlayerEvent, PlayerState, RuntimeEvent, TileApplyEvent, TileEvent,
};

pub trait SessionEventApplier {
    fn handles_event(&self, event: &RuntimeEvent) -> bool;
    fn apply_runtime_event(&mut self, event: &RuntimeEvent) -> bool;
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

    pub fn apply_dialog_action(&mut self, data: &GameData, action: &crate::data::DialogAction) {
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
            crate::data::DialogAction::OpenShop(_) => {}
            crate::data::DialogAction::Heal => {
                self.player.stats.current_hp = self.player.stats.max_hp;
                self.player.stats.current_mp = self.player.stats.max_mp;
            }
        }
    }
}

struct SessionLifecycleApplier;

static SESSION_LIFECYCLE_APPLIER: SessionLifecycleApplier = SessionLifecycleApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&SESSION_LIFECYCLE_APPLIER]
}

impl DomainEventApplier for SessionLifecycleApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::StartNewGame
                | RuntimeEvent::ContinueGame
                | RuntimeEvent::RestoreSessionStats
                | RuntimeEvent::ApplyDialogAction(_)
                | RuntimeEvent::Transition(crate::game::TransitionEvent::MapChanged)
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        match event {
            RuntimeEvent::StartNewGame => {
                let (state, session) = start_new_game(ctx.data);
                enter_session(ctx, state, session);
            }
            RuntimeEvent::ContinueGame => {
                let (state, session) = continue_game(ctx.data);
                enter_session(ctx, state, session);
            }
            RuntimeEvent::RestoreSessionStats => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.restore_stats();
            }
            RuntimeEvent::ApplyDialogAction(action) => {
                let data = ctx.data_rc();
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.apply_dialog_action(&data, action);
            }
            RuntimeEvent::Transition(crate::game::TransitionEvent::MapChanged) => {
                let data = ctx.data_rc();
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.spawn_current_map_enemies(&data);
            }
            _ => {}
        }
        Ok(())
    }
}

fn start_new_game(data: &GameData) -> (crate::game::GameState, SessionState) {
    let config = &data.newgame;
    let mut player = PlayerState::new(config.player_name.clone(), &config.start_map);
    let combat = CombatState::default();

    if let Some(ref weapon_id) = config.equip_weapon
        && let Some(weapon) = data.find_item(weapon_id).cloned()
    {
        let idx = player.inventory.len();
        player.inventory.push(weapon);
        player.equipped_weapon = Some(idx);
    }
    if let Some(ref armor_id) = config.equip_armor
        && let Some(armor) = data.find_item(armor_id).cloned()
    {
        let idx = player.inventory.len();
        player.inventory.push(armor);
        player.equipped_armor = Some(idx);
    }
    for start_item in &config.items {
        if let Some(item) = data.find_item(&start_item.item_id).cloned() {
            for _ in 0..start_item.count {
                player.inventory.push(item.clone());
            }
        }
    }

    if let Some(map) = data.find_map(&config.start_map) {
        let (x, y) = map.find_player_start().unwrap_or((player.x, player.y));
        player.current_map_id = map.id.clone();
        player.x = x;
        player.y = y;
    }

    let state = if config
        .intro_dialog
        .as_ref()
        .and_then(|(dialog_id, _)| data.find_dialog(dialog_id))
        .is_some()
    {
        crate::game::GameState::Dialog
    } else {
        crate::game::GameState::Explore
    };

    let session = SessionState {
        player,
        combat,
        movement: MovementState::default(),
        skill_cooldowns: [0; 3],
        mp_regen_timer: 0,
    };

    (state, session)
}

fn continue_game(data: &GameData) -> (crate::game::GameState, SessionState) {
    let config = &data.newgame;
    let mut player = PlayerState::new(config.player_name.clone(), &config.start_map);
    let combat = CombatState::default();

    match crate::game::load_game(&mut player) {
        Ok(true) => {
            if data.find_map(&player.current_map_id).is_none() {
                let (x, y) = (player.x, player.y);
                player.current_map_id = config.fallback_map.clone();
                player.x = x;
                player.y = y;
            }
            if let Some(map) = data.find_map(&player.current_map_id)
                && (map.get_tile(player.x, player.y) == Tile::Wall
                    || player.x >= map.width
                    || player.y >= map.height)
                && let Some((x, y)) = map.find_player_start()
            {
                player.x = x;
                player.y = y;
            }

            let session = SessionState {
                player,
                combat,
                movement: MovementState::default(),
                skill_cooldowns: [0; 3],
                mp_regen_timer: 0,
            };

            (crate::game::GameState::Explore, session)
        }
        Ok(false) | Err(_) => start_new_game(data),
    }
}

fn enter_session(ctx: &mut ApplyContext<'_>, state: crate::game::GameState, session: SessionState) {
    *ctx.session = Some(session);
    ctx.transition_to(state);

    if let Some(s) = ctx.session.as_mut() {
        s.spawn_current_map_enemies(ctx.data);
    }
}

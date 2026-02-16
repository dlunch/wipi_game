use crate::game::{
    CombatAction, CombatEvent, CombatState, GameData, MovementState, MovementTickEvent,
    PlayerAction, PlayerEffect, PlayerEvent, PlayerState, TileApplyEvent, TileEvent,
};

pub struct SessionState {
    pub player: PlayerState,
    pub combat: CombatState,
    pub movement: MovementState,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
}

impl SessionState {
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

    pub fn apply_combat_tick(&mut self, event: crate::game::combat::CombatTickEvent) -> bool {
        let crate::game::combat::CombatTickEvent {
            damage_taken,
            next_skill_cooldowns,
            next_mp_regen_timer,
            recover_mp,
            next_state,
        } = event;

        self.combat = next_state;
        self.skill_cooldowns = next_skill_cooldowns;
        self.mp_regen_timer = next_mp_regen_timer;
        if recover_mp > 0 {
            self.player.stats.recover_mp(recover_mp);
        }

        if damage_taken > 0
            && matches!(
                self.player.apply(PlayerAction::TakeDamage(damage_taken)),
                PlayerEvent::Died
            )
        {
            return true;
        }

        false
    }

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
}

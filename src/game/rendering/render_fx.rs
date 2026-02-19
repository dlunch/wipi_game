use alloc::{string::String, vec::Vec};

use anyhow::{Result, anyhow};

use crate::{
    data::{Direction, Skill, SkillType},
    game::{
        game_event::{CombatEvent, EntityEvent, GameEvent, WorldEvent},
        state::{GameState, TimedKind},
        world::WorldState,
    },
};

#[derive(Default)]
pub struct RenderFxState {
    player_hit_flash: u32,
    enemy_hit_flashes: Vec<(u32, u32)>,
    skill_effects: Vec<SkillEffectInstance>,
    quest_notice_timer: u32,
    shop_purchase_notice_timer: u32,
    soft_error_notice_timer: u32,
    soft_error_message: Option<String>,
    anim_tick: u32,
}

#[derive(Clone, Copy)]
struct SkillEffectInstance {
    x: usize,
    y: usize,
    effect_type: SkillType,
    timer: u32,
}

const HIT_FLASH_DURATION: u32 = 10;
const SKILL_EFFECT_DURATION: u32 = 8;
const QUEST_NOTICE_DURATION: u32 = 90;
const SHOP_PURCHASE_NOTICE_DURATION: u32 = 45;
const SOFT_ERROR_NOTICE_DURATION: u32 = 60;

impl RenderFxState {
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        self.anim_tick = self.anim_tick.wrapping_add(1);
        if self.player_hit_flash > 0 {
            self.player_hit_flash -= 1;
            changed = true;
        }

        for (_, timer) in &mut self.enemy_hit_flashes {
            if *timer > 0 {
                *timer -= 1;
                changed = true;
            }
        }
        let before = self.enemy_hit_flashes.len();
        self.enemy_hit_flashes.retain(|(_, timer)| *timer > 0);
        let before_skill = self.skill_effects.len();
        for effect in &mut self.skill_effects {
            if effect.timer > 0 {
                effect.timer -= 1;
                changed = true;
            }
        }
        self.skill_effects.retain(|effect| effect.timer > 0);
        if self.quest_notice_timer > 0 {
            self.quest_notice_timer -= 1;
            changed = true;
        }
        if self.shop_purchase_notice_timer > 0 {
            self.shop_purchase_notice_timer -= 1;
            changed = true;
        }
        if self.soft_error_notice_timer > 0 {
            self.soft_error_notice_timer -= 1;
            changed = true;
            if self.soft_error_notice_timer == 0 {
                self.soft_error_message = None;
            }
        }
        changed
            || before != self.enemy_hit_flashes.len()
            || before_skill != self.skill_effects.len()
    }

    pub fn apply_event(
        &mut self,
        state: &GameState,
        world: Option<&WorldState>,
        event: &GameEvent,
    ) -> Result<bool> {
        let changed = match event {
            GameEvent::Entity(EntityEvent::ChangeEntityHp { entity_id, delta }) if *delta < 0 => {
                let Some(world) = world else {
                    return Ok(false);
                };
                if *entity_id == world.leader_id()? {
                    let changed = self.player_hit_flash != HIT_FLASH_DURATION;
                    self.player_hit_flash = HIT_FLASH_DURATION;
                    changed
                } else {
                    if let Some((_, timer)) = self
                        .enemy_hit_flashes
                        .iter_mut()
                        .find(|(id, _)| *id == *entity_id)
                    {
                        let changed = *timer != HIT_FLASH_DURATION;
                        *timer = HIT_FLASH_DURATION;
                        return Ok(changed);
                    }
                    self.enemy_hit_flashes
                        .push((*entity_id, HIT_FLASH_DURATION));
                    true
                }
            }
            GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id,
                kind: TimedKind::SkillCooldown(slot),
                end_tick,
            }) if matches!(
                state,
                GameState::Explore | GameState::Dialog | GameState::PauseMenu
            ) =>
            {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                if *end_tick <= world.tick_counter {
                    return Ok(false);
                }
                let leader_id = world.leader_id()?;
                if *entity_id != leader_id
                    || !is_skill_cast_cooldown(*slot, *end_tick, world.tick_counter)
                {
                    return Ok(false);
                }
                self.skill_effects.clear();
                push_skill_effects(world, *slot, &mut self.skill_effects)?;
                !self.skill_effects.is_empty()
            }
            GameEvent::World(WorldEvent::CreateQuestProgress { .. })
                if matches!(state, GameState::Explore | GameState::Dialog) =>
            {
                let changed = self.quest_notice_timer != QUEST_NOTICE_DURATION;
                self.quest_notice_timer = QUEST_NOTICE_DURATION;
                changed
            }
            GameEvent::ShopBuyItem(_) if matches!(state, GameState::Shop) => {
                let changed = self.shop_purchase_notice_timer != SHOP_PURCHASE_NOTICE_DURATION;
                self.shop_purchase_notice_timer = SHOP_PURCHASE_NOTICE_DURATION;
                changed
            }
            GameEvent::SoftError(message) => {
                let changed = self.soft_error_notice_timer != SOFT_ERROR_NOTICE_DURATION
                    || self.soft_error_message.as_deref() != Some(message.as_str());
                self.soft_error_notice_timer = SOFT_ERROR_NOTICE_DURATION;
                self.soft_error_message = Some(message.clone());
                changed
            }
            _ => false,
        };
        Ok(changed)
    }

    pub fn player_hit_flash(&self) -> u32 {
        self.player_hit_flash
    }

    pub fn quest_notice_timer(&self) -> u32 {
        self.quest_notice_timer
    }

    pub fn shop_purchase_notice_timer(&self) -> u32 {
        self.shop_purchase_notice_timer
    }

    pub fn soft_error_notice_timer(&self) -> u32 {
        self.soft_error_notice_timer
    }

    pub fn soft_error_message(&self) -> Option<&str> {
        self.soft_error_message.as_deref()
    }

    pub fn anim_tick(&self) -> u32 {
        self.anim_tick
    }

    pub fn enemy_hit_flash(&self, enemy_id: u32) -> u32 {
        for (id, timer) in &self.enemy_hit_flashes {
            if *id == enemy_id {
                return *timer;
            }
        }
        0
    }

    pub fn skill_effect_iter(&self) -> impl Iterator<Item = (usize, usize, SkillType)> + '_ {
        self.skill_effects
            .iter()
            .map(|effect| (effect.x, effect.y, effect.effect_type))
    }
}

fn is_skill_cast_cooldown(slot: u8, end_tick: u32, current_tick: u32) -> bool {
    if end_tick <= current_tick {
        return false;
    }
    let time_left = end_tick - current_tick;
    match slot {
        0 => time_left == Skill::FIREBALL.cooldown,
        1 => time_left == Skill::HEAL.cooldown,
        2 => time_left == Skill::SPIN_ATTACK.cooldown,
        _ => false,
    }
}

fn push_skill_effect(
    out: &mut Vec<SkillEffectInstance>,
    x: usize,
    y: usize,
    effect_type: SkillType,
) {
    if out.iter().any(|effect| effect.x == x && effect.y == y) {
        return;
    }
    out.push(SkillEffectInstance {
        x,
        y,
        effect_type,
        timer: SKILL_EFFECT_DURATION,
    });
}

fn push_skill_effects(
    world: &WorldState,
    slot: u8,
    out: &mut Vec<SkillEffectInstance>,
) -> Result<()> {
    let leader = world.leader_entity()?;
    match slot {
        0 => {
            for dist in 1..=Skill::FIREBALL.range {
                let (x, y) = leader.facing.apply_distance(leader.x, leader.y, dist);
                push_skill_effect(out, x, y, SkillType::Ranged);
            }
        }
        1 => {
            push_skill_effect(out, leader.x, leader.y, SkillType::Heal);
        }
        2 => {
            for direction in [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                let (x, y) = direction.apply(leader.x, leader.y);
                push_skill_effect(out, x, y, SkillType::Area);
            }
        }
        _ => {}
    }
    Ok(())
}

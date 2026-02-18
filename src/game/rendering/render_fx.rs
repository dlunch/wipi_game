use alloc::vec::Vec;

use crate::data::{Direction, Skill, SkillType};
use crate::game::state::TimedKind;
use crate::game::{CombatEvent, GameEvent, GameState, WorldEvent, WorldState};

#[derive(Default)]
pub struct RenderFxState {
    player_hit_flash: u32,
    enemy_hit_flashes: Vec<(u32, u32)>,
    skill_effects: Vec<SkillEffectInstance>,
    quest_notice_timer: u32,
    shop_purchase_notice_timer: u32,
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
        changed
            || before != self.enemy_hit_flashes.len()
            || before_skill != self.skill_effects.len()
    }

    pub fn apply_event(
        &mut self,
        state: &GameState,
        world: Option<&WorldState>,
        event: &GameEvent,
    ) -> bool {
        match event {
            GameEvent::Combat(CombatEvent::TakeDamage { entity_id, amount }) if *amount > 0 => {
                let leader_id = world.and_then(|w| w.leader_id());
                if Some(*entity_id) == leader_id {
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
                        return changed;
                    }
                    self.enemy_hit_flashes
                        .push((*entity_id, HIT_FLASH_DURATION));
                    true
                }
            }
            GameEvent::Combat(CombatEvent::SetCombatantTimed {
                entity_id,
                kind: TimedKind::SkillCooldown(slot),
                time_left,
            }) if matches!(
                state,
                GameState::Explore | GameState::Dialog | GameState::PauseMenu
            ) =>
            {
                if *time_left == 0 {
                    return false;
                }
                let Some(world) = world else {
                    return false;
                };
                let Some(leader_id) = world.leader_id() else {
                    return false;
                };
                if *entity_id != leader_id || !is_skill_cast_cooldown(*slot, *time_left) {
                    return false;
                }
                self.skill_effects.clear();
                push_skill_effects(world, *slot, &mut self.skill_effects);
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
            _ => false,
        }
    }

    fn enemy_hit_flash_internal(&self, enemy_id: u32) -> u32 {
        self.enemy_hit_flashes
            .iter()
            .find_map(|(id, timer)| (*id == enemy_id).then_some(*timer))
            .unwrap_or(0)
    }

    pub(super) fn player_hit_flash(&self) -> u32 {
        self.player_hit_flash
    }

    pub(super) fn quest_notice_timer(&self) -> u32 {
        self.quest_notice_timer
    }

    pub(super) fn shop_purchase_notice_timer(&self) -> u32 {
        self.shop_purchase_notice_timer
    }

    pub(super) fn anim_tick(&self) -> u32 {
        self.anim_tick
    }

    pub(super) fn enemy_hit_flash_value(&self, enemy_id: u32) -> u32 {
        self.enemy_hit_flash_internal(enemy_id)
    }

    pub(super) fn enemy_hit_flash(&self, enemy_id: u32) -> u32 {
        self.enemy_hit_flash_value(enemy_id)
    }

    pub(super) fn skill_effect_iter(&self) -> impl Iterator<Item = (usize, usize, SkillType)> + '_ {
        self.skill_effects
            .iter()
            .map(|effect| (effect.x, effect.y, effect.effect_type))
    }
}

fn is_skill_cast_cooldown(slot: u8, time_left: u32) -> bool {
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

fn push_skill_effects(world: &WorldState, slot: u8, out: &mut Vec<SkillEffectInstance>) {
    let Some(leader) = world.leader_entity() else {
        return;
    };
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
}

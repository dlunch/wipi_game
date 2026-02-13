use crate::data::ItemKind;
use crate::game::PlayerState;

pub enum PlayerIntent {
    UpdateCooldowns,
    UseSkill {
        slot: usize,
        mp_cost: i32,
        cooldown: u32,
    },
    UseItem {
        index: usize,
    },
    TakeDamage(i32),
    Heal(i32),
    RecoverMp(i32),
}

pub enum PlayerEvent {
    None,
    ItemUsed,
    SkillUsed,
    Died,
}

pub fn reduce(player: &mut PlayerState, intent: PlayerIntent) -> PlayerEvent {
    match intent {
        PlayerIntent::UpdateCooldowns => {
            update_cooldowns(player);
            PlayerEvent::None
        }
        PlayerIntent::UseSkill {
            slot,
            mp_cost,
            cooldown,
        } => {
            if can_use_skill(player, slot, mp_cost) {
                use_skill(player, slot, mp_cost, cooldown);
                PlayerEvent::SkillUsed
            } else {
                PlayerEvent::None
            }
        }
        PlayerIntent::UseItem { index } => {
            if use_item(player, index) {
                PlayerEvent::ItemUsed
            } else {
                PlayerEvent::None
            }
        }
        PlayerIntent::TakeDamage(amount) => {
            player.stats.take_damage(amount);
            if player.stats.is_dead() {
                PlayerEvent::Died
            } else {
                PlayerEvent::None
            }
        }
        PlayerIntent::Heal(amount) => {
            player.stats.heal(amount);
            PlayerEvent::None
        }
        PlayerIntent::RecoverMp(amount) => {
            player.stats.recover_mp(amount);
            PlayerEvent::None
        }
    }
}

pub fn update_cooldowns(player: &mut PlayerState) {
    for cd in &mut player.skill_cooldowns {
        if *cd > 0 {
            *cd -= 1;
        }
    }
}

pub fn can_use_skill(player: &PlayerState, slot: usize, mp_cost: i32) -> bool {
    slot < 3 && player.skill_cooldowns[slot] == 0 && player.stats.current_mp >= mp_cost
}

pub fn use_skill(player: &mut PlayerState, slot: usize, mp_cost: i32, cooldown: u32) {
    if slot < 3 {
        player.skill_cooldowns[slot] = cooldown;
        player.stats.current_mp = (player.stats.current_mp - mp_cost).max(0);
    }
}

pub fn use_item(player: &mut PlayerState, index: usize) -> bool {
    if index >= player.inventory.len() {
        return false;
    }

    let item = &player.inventory[index];
    match item.kind {
        ItemKind::Consumable => {
            let heal = item.hp_restore();
            player.stats.heal(heal);
            player.inventory.remove(index);
            player.fix_equipped_indices(index);
            true
        }
        ItemKind::Weapon => {
            player.equipped_weapon = Some(index);
            true
        }
        ItemKind::Armor => {
            player.equipped_armor = Some(index);
            true
        }
        ItemKind::Accessory => {
            player.equipped_accessory = Some(index);
            true
        }
    }
}

use alloc::string::String;

use crate::data::Item;
use crate::data::ItemKind;
use crate::game::PlayerState;

const MP_REGEN_INTERVAL: u32 = 60;

#[derive(Debug, Clone)]
pub enum PlayerIntent {
    AddExp(i32),
    AddGold(i32),
    FullHeal,
    AddItem(Item),
    RemoveItem(String),
    RemoveItemAt(usize),
    EquipWeapon(usize),
    EquipArmor(usize),
    SetFacing {
        dx: i32,
        dy: i32,
    },
    MoveBy {
        dx: i32,
        dy: i32,
    },
    ChangeMap {
        map_id: String,
        x: usize,
        y: usize,
    },
    SpawnAtMap {
        x: usize,
        y: usize,
    },
    OpenTreasure {
        map_id: String,
        x: usize,
        y: usize,
    },
    AddQuest(String),
    MarkQuestRewarded(String),
    UpdateQuestProgress {
        quest_id: String,
        target_count: i32,
    },
    TickMpRegen,
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

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    None,
    ItemUsed,
    SkillUsed,
    Died,
    ItemRemoved(Option<Item>),
}

pub fn reduce(player: &mut PlayerState, intent: PlayerIntent) -> PlayerEvent {
    match intent {
        PlayerIntent::AddExp(exp) => {
            player.stats.add_exp(exp);
            PlayerEvent::None
        }
        PlayerIntent::AddGold(amount) => {
            player.stats.gold = (player.stats.gold + amount).max(0);
            PlayerEvent::None
        }
        PlayerIntent::FullHeal => {
            player.stats.current_hp = player.stats.max_hp;
            player.stats.current_mp = player.stats.max_mp;
            PlayerEvent::None
        }
        PlayerIntent::AddItem(item) => {
            player.add_item(item);
            PlayerEvent::None
        }
        PlayerIntent::RemoveItem(id) => {
            player.remove_item(&id);
            PlayerEvent::None
        }
        PlayerIntent::RemoveItemAt(index) => PlayerEvent::ItemRemoved(player.remove_item_at(index)),
        PlayerIntent::EquipWeapon(idx) => {
            player.equipped_weapon = Some(idx);
            PlayerEvent::None
        }
        PlayerIntent::EquipArmor(idx) => {
            player.equipped_armor = Some(idx);
            PlayerEvent::None
        }
        PlayerIntent::SetFacing { dx, dy } => {
            player.set_facing(dx, dy);
            PlayerEvent::None
        }
        PlayerIntent::MoveBy { dx, dy } => {
            player.move_by(dx, dy);
            PlayerEvent::None
        }
        PlayerIntent::ChangeMap { map_id, x, y } => {
            player.current_map_id = map_id;
            player.x = x;
            player.y = y;
            PlayerEvent::None
        }
        PlayerIntent::SpawnAtMap { x, y } => {
            player.x = x;
            player.y = y;
            PlayerEvent::None
        }
        PlayerIntent::OpenTreasure { map_id, x, y } => {
            player.open_treasure(&map_id, x, y);
            PlayerEvent::None
        }
        PlayerIntent::AddQuest(id) => {
            player.add_quest(&id);
            PlayerEvent::None
        }
        PlayerIntent::MarkQuestRewarded(id) => {
            player.mark_quest_rewarded(&id);
            PlayerEvent::None
        }
        PlayerIntent::UpdateQuestProgress {
            quest_id,
            target_count,
        } => {
            for progress in &mut player.quests {
                if progress.quest_id == quest_id && !progress.completed {
                    progress.current_count += 1;
                    if progress.current_count >= target_count {
                        progress.completed = true;
                    }
                }
            }
            PlayerEvent::None
        }
        PlayerIntent::TickMpRegen => {
            player.mp_regen_timer += 1;
            if player.mp_regen_timer >= MP_REGEN_INTERVAL {
                player.mp_regen_timer = 0;
                let _ = reduce(player, PlayerIntent::RecoverMp(1));
            }
            PlayerEvent::None
        }
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

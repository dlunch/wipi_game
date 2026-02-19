use alloc::{rc::Rc, string::String, vec, vec::Vec};

use anyhow::{Result, anyhow};

use super::resolver::DomainEventResolver;
use crate::{
    data::{DialogAction, ItemKind},
    game::{
        game_data::GameData,
        game_event::{EntityEvent, GameEvent, GameEventKind, LoadoutSlot},
        state::{EntityState, GOLD_ITEM_ID},
        world::WorldState,
    },
};

struct CharacterMutationResolver;

static CHARACTER_MUTATION_RESOLVER: CharacterMutationResolver = CharacterMutationResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&CHARACTER_MUTATION_RESOLVER]
}

impl DomainEventResolver for CharacterMutationResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[
            GameEventKind::UseInventorySelected,
            GameEventKind::ShopBuyItem,
            GameEventKind::ShopSellItem,
            GameEventKind::RestoreHpMp,
            GameEventKind::ApplyDialogAction,
        ]
    }

    fn resolve(
        &self,
        data: &Rc<GameData>,
        world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let world = world.ok_or_else(|| anyhow!("No active world"))?;
        let leader_id = world.leader_id()?;
        let leader = world.entity(leader_id)?;

        match event {
            GameEvent::UseInventorySelected(index) => {
                resolve_use_item(data, leader_id, leader, *index, out)?
            }
            GameEvent::ShopBuyItem(item_id) => {
                resolve_shop_buy(data, world, leader_id, *item_id, out)?
            }
            GameEvent::ShopSellItem(item_data_id) => {
                resolve_shop_sell(data, leader_id, leader, *item_data_id, out)?
            }
            GameEvent::RestoreHpMp => resolve_restore_hp_mp(world, leader_id, out)?,
            GameEvent::ApplyDialogAction(action) => {
                resolve_dialog_action(world, leader_id, action, out)?
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_use_item(
    data: &GameData,
    leader_id: u32,
    leader: &EntityState,
    index: usize,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let Some(stack) = leader.inventory.get(index) else {
        push_soft_error(out, "Invalid inventory selection");
        return Ok(());
    };
    if stack.amount <= 0 {
        push_soft_error(out, "Item is unavailable");
        return Ok(());
    }

    let item = data.find_item(&stack.item_id)?;
    if let Some(slot) = loadout_slot(item.kind) {
        out.push(GameEvent::Entity(EntityEvent::SetEntityLoadoutSlot {
            entity_id: leader_id,
            slot,
            index: Some(index),
        }));
        return Ok(());
    }

    out.push(GameEvent::Entity(EntityEvent::ChangeEntityHp {
        entity_id: leader_id,
        delta: item.hp_restore(),
    }));
    push_item_delta(out, leader_id, stack.item_id.clone(), -1);
    Ok(())
}

fn resolve_shop_buy(
    data: &GameData,
    world: &WorldState,
    leader_id: u32,
    item_data_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let item = data.find_item_by_data_id(item_data_id)?;
    if world.gold_amount(leader_id)? < item.price {
        push_soft_error(out, "Not enough gold");
        return Ok(());
    }

    push_item_delta(out, leader_id, GOLD_ITEM_ID, -item.price.max(0));
    push_item_delta(out, leader_id, item.id.clone(), 1);
    Ok(())
}

fn resolve_shop_sell(
    data: &GameData,
    leader_id: u32,
    leader: &EntityState,
    item_data_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let item = data.find_item_by_data_id(item_data_id)?;
    if item.id == GOLD_ITEM_ID {
        push_soft_error(out, "Cannot sell gold");
        return Ok(());
    }

    let Some(stack) = leader
        .inventory
        .iter()
        .find(|stack| stack.item_id == item.id && stack.amount > 0)
    else {
        push_soft_error(out, "Invalid item selection");
        return Ok(());
    };

    push_item_delta(out, leader_id, stack.item_id.clone(), -1);
    push_item_delta(out, leader_id, GOLD_ITEM_ID, (item.price / 2).max(0));
    Ok(())
}

fn resolve_restore_hp_mp(
    world: &WorldState,
    entity_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let entity = world.entity(entity_id)?;
    let hp_delta = entity.stat.base_max_hp - entity.current_hp;
    let mp_delta = entity.stat.base_max_mp - entity.current_mp;
    if hp_delta != 0 {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityHp {
            entity_id,
            delta: hp_delta,
        }));
    }
    if mp_delta != 0 {
        out.push(GameEvent::Entity(EntityEvent::ChangeEntityMp {
            entity_id,
            delta: mp_delta,
        }));
    }
    Ok(())
}

fn resolve_dialog_action(
    world: &WorldState,
    entity_id: u32,
    action: &DialogAction,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    match action {
        DialogAction::GiveItem(id) => {
            push_item_delta(out, entity_id, id.clone(), 1);
        }
        DialogAction::TakeItem(id) => {
            push_item_delta(out, entity_id, id.clone(), -1);
        }
        DialogAction::GiveGold(amount) => {
            push_item_delta(out, entity_id, GOLD_ITEM_ID, (*amount).max(0));
        }
        DialogAction::TakeGold(amount) => {
            push_item_delta(out, entity_id, GOLD_ITEM_ID, -(*amount).max(0));
        }
        DialogAction::Heal => resolve_restore_hp_mp(world, entity_id, out)?,
        DialogAction::GiveQuest(_) | DialogAction::CompleteQuest(_) | DialogAction::OpenShop(_) => {
        }
    }
    Ok(())
}

fn push_item_delta(
    out: &mut Vec<GameEvent>,
    entity_id: u32,
    item_id: impl Into<String>,
    delta: i32,
) {
    out.push(GameEvent::Entity(EntityEvent::ChangeEntityItem {
        entity_id,
        item_id: item_id.into(),
        delta,
    }));
}

fn push_soft_error(out: &mut Vec<GameEvent>, message: &str) {
    out.push(GameEvent::SoftError(String::from(message)));
}

fn loadout_slot(kind: ItemKind) -> Option<LoadoutSlot> {
    match kind {
        ItemKind::Weapon => Some(LoadoutSlot::Weapon),
        ItemKind::Armor => Some(LoadoutSlot::Armor),
        ItemKind::Accessory => Some(LoadoutSlot::Accessory),
        ItemKind::Consumable => None,
    }
}

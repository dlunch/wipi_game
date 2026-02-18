use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{DialogAction, ItemKind};
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{
    CombatEvent, GameData, GameEvent, GameEventKind, LoadoutSlot, WorldEvent, WorldState,
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
            GameEventKind::ShopSellSelected,
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
        let leader_id = world
            .leader_id()
            .ok_or_else(|| anyhow!("No leader entity"))?;
        let leader = world
            .entity(leader_id)
            .ok_or_else(|| anyhow!("Leader entity not found"))?;

        match event {
            GameEvent::UseInventorySelected(index) => {
                resolve_use_item(data, world, leader_id, leader, *index, out)
            }
            GameEvent::ShopBuyItem(item_id) => {
                resolve_shop_buy(data, world, leader_id, item_id, out)
            }
            GameEvent::ShopSellSelected(index) => {
                resolve_shop_sell(data, world, leader_id, leader, *index, out)
            }
            GameEvent::RestoreHpMp => resolve_restore_hp_mp(world, leader_id, out),
            GameEvent::ApplyDialogAction(action) => {
                resolve_dialog_action(data, world, leader_id, action, out)?
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_use_item(
    data: &GameData,
    world: &WorldState,
    leader_id: u32,
    leader: &crate::game::EntityState,
    index: usize,
    out: &mut Vec<GameEvent>,
) {
    let Some(stack) = leader.inventory.get(index) else {
        return;
    };
    if stack.amount <= 0 {
        return;
    }

    let Some(item) = data.find_item(&stack.item_id) else {
        return;
    };
    match item.kind {
        ItemKind::Consumable => {
            out.push(GameEvent::Combat(CombatEvent::Heal {
                entity_id: leader_id,
                amount: item.hp_restore(),
            }));
            out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
                entity_id: leader_id,
                item_id: stack.item_id.clone(),
                delta: -1,
            }));
        }
        ItemKind::Weapon => {
            out.push(GameEvent::World(WorldEvent::SetEntityLoadoutSlot {
                entity_id: leader_id,
                slot: LoadoutSlot::Weapon,
                index: Some(index),
            }));
        }
        ItemKind::Armor => {
            out.push(GameEvent::World(WorldEvent::SetEntityLoadoutSlot {
                entity_id: leader_id,
                slot: LoadoutSlot::Armor,
                index: Some(index),
            }));
        }
        ItemKind::Accessory => {
            out.push(GameEvent::World(WorldEvent::SetEntityLoadoutSlot {
                entity_id: leader_id,
                slot: LoadoutSlot::Accessory,
                index: Some(index),
            }));
        }
    }
    sync_leader_combat_stats(data, world, leader_id, out);
}

fn resolve_shop_buy(
    data: &GameData,
    world: &WorldState,
    leader_id: u32,
    item_id: &str,
    out: &mut Vec<GameEvent>,
) {
    let Some(item) = data.find_item(item_id) else {
        return;
    };
    if world.gold_amount(leader_id) < item.price {
        return;
    }

    out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
        entity_id: leader_id,
        item_id: crate::game::GOLD_ITEM_ID.into(),
        delta: -item.price.max(0),
    }));
    out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
        entity_id: leader_id,
        item_id: item_id.into(),
        delta: 1,
    }));
}

fn resolve_shop_sell(
    data: &GameData,
    world: &WorldState,
    leader_id: u32,
    leader: &crate::game::EntityState,
    index: usize,
    out: &mut Vec<GameEvent>,
) {
    let Some(stack) = leader.inventory.get(index) else {
        return;
    };
    if stack.amount <= 0 || stack.item_id == crate::game::GOLD_ITEM_ID {
        return;
    }
    let Some(item) = data.find_item(&stack.item_id) else {
        return;
    };

    out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
        entity_id: leader_id,
        item_id: stack.item_id.clone(),
        delta: -1,
    }));
    out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
        entity_id: leader_id,
        item_id: crate::game::GOLD_ITEM_ID.into(),
        delta: (item.price / 2).max(0),
    }));
    sync_leader_combat_stats(data, world, leader_id, out);
}

fn resolve_restore_hp_mp(world: &WorldState, entity_id: u32, out: &mut Vec<GameEvent>) {
    let Some(combatant) = world.combat.combatant(entity_id) else {
        return;
    };
    let mut stats = combatant.stats;
    stats.current_hp = stats.max_hp;
    stats.current_mp = stats.max_mp;
    out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
        entity_id,
        stats,
    }));
}

fn resolve_dialog_action(
    data: &GameData,
    world: &WorldState,
    entity_id: u32,
    action: &DialogAction,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    match action {
        DialogAction::GiveItem(id) => {
            out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
                entity_id,
                item_id: id.clone(),
                delta: 1,
            }));
            sync_leader_combat_stats(data, world, entity_id, out);
        }
        DialogAction::TakeItem(id) => {
            out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
                entity_id,
                item_id: id.clone(),
                delta: -1,
            }));
            sync_leader_combat_stats(data, world, entity_id, out);
        }
        DialogAction::GiveGold(amount) => {
            out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
                entity_id,
                item_id: crate::game::GOLD_ITEM_ID.into(),
                delta: (*amount).max(0),
            }));
        }
        DialogAction::TakeGold(amount) => {
            out.push(GameEvent::World(WorldEvent::ChangeEntityItem {
                entity_id,
                item_id: crate::game::GOLD_ITEM_ID.into(),
                delta: -(*amount).max(0),
            }));
        }
        DialogAction::Heal => resolve_restore_hp_mp(world, entity_id, out),
        DialogAction::GiveQuest(_) | DialogAction::CompleteQuest(_) | DialogAction::OpenShop(_) => {
        }
    }
    Ok(())
}

fn sync_leader_combat_stats(
    data: &GameData,
    world: &WorldState,
    entity_id: u32,
    out: &mut Vec<GameEvent>,
) {
    let Some(entity) = world.entity(entity_id) else {
        return;
    };
    let Some(combatant) = world.combat.combatant(entity_id) else {
        return;
    };
    let mut stats = combatant.stats;
    let mut atk = entity.stat.base_atk;
    let mut def = entity.stat.base_def;
    if let Some(index) = entity.loadout.weapon
        && let Some(stack) = entity.inventory.get(index)
        && let Some(item) = data.find_item(&stack.item_id)
    {
        atk += item.atk();
    }
    if let Some(index) = entity.loadout.armor
        && let Some(stack) = entity.inventory.get(index)
        && let Some(item) = data.find_item(&stack.item_id)
    {
        def += item.def();
    }
    if let Some(index) = entity.loadout.accessory
        && let Some(stack) = entity.inventory.get(index)
        && let Some(item) = data.find_item(&stack.item_id)
    {
        atk += item.atk();
        def += item.def();
    }
    stats.atk = atk;
    stats.def = def;
    out.push(GameEvent::Combat(CombatEvent::SetCombatantStats {
        entity_id,
        stats,
    }));
}

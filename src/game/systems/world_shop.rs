use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{
    game_data::GameData,
    game_event::{GameEvent, ShopItemEntry, ShopItemListKind},
    state::GOLD_ITEM_ID,
    world::WorldState,
};

pub fn resolve_open_shop(
    data: &GameData,
    world: &WorldState,
    shop_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let shop = data.find_shop(shop_id)?;
    let mut buy_items = Vec::with_capacity(shop.items.len());
    for item_id in &shop.items {
        buy_items.push(ShopItemEntry {
            item_id: *item_id,
            amount: 1,
        });
    }
    out.push(GameEvent::SetShopItems {
        list: ShopItemListKind::Buy,
        items: buy_items,
    });
    out.push(GameEvent::SetShopItems {
        list: ShopItemListKind::Sell,
        items: sell_item_entries(world)?,
    });
    Ok(())
}

pub fn resolve_shop_sell_cache_after_buy(
    data: &GameData,
    world: &WorldState,
    item_data_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let leader_id = world.leader_id()?;
    let item = data.find_item(item_data_id)?;
    if world.gold_amount(leader_id)? < item.price {
        return Ok(());
    }

    let mut sell_items = sell_item_entries(world)?;
    if let Some(entry) = sell_items
        .iter_mut()
        .find(|entry| entry.item_id == item_data_id)
    {
        entry.amount += 1;
    } else {
        sell_items.push(ShopItemEntry {
            item_id: item_data_id,
            amount: 1,
        });
    }
    out.push(GameEvent::SetShopItems {
        list: ShopItemListKind::Sell,
        items: sell_items,
    });
    Ok(())
}

pub fn resolve_shop_sell_cache_after_sell(
    data: &GameData,
    world: &WorldState,
    item_data_id: u32,
    out: &mut Vec<GameEvent>,
) -> Result<()> {
    let item = data.find_item(item_data_id)?;
    if item.id == GOLD_ITEM_ID {
        return Ok(());
    }

    let leader = world.leader_entity()?;
    let has_item = leader
        .inventory
        .iter()
        .any(|stack| stack.item_id == item.id && stack.amount > 0);
    if !has_item {
        return Ok(());
    }

    let mut sell_items = sell_item_entries(world)?;
    if let Some(index) = sell_items
        .iter()
        .position(|entry| entry.item_id == item_data_id)
    {
        if let Some(entry) = sell_items.get_mut(index) {
            entry.amount -= 1;
        }
        if let Some(entry) = sell_items.get(index)
            && entry.amount <= 0
        {
            sell_items.remove(index);
        }
    }
    out.push(GameEvent::SetShopItems {
        list: ShopItemListKind::Sell,
        items: sell_items,
    });
    Ok(())
}

fn sell_item_entries(world: &WorldState) -> Result<Vec<ShopItemEntry>> {
    let leader = world.leader_entity()?;
    let mut sell_items = Vec::new();
    for stack in &leader.inventory {
        if stack.item_id == GOLD_ITEM_ID || stack.amount <= 0 {
            continue;
        }
        sell_items.push(ShopItemEntry {
            item_id: stack.item_id,
            amount: stack.amount,
        });
    }
    Ok(sell_items)
}

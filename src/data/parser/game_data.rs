use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use anyhow::{Result, bail, ensure};

use super::{parse_int, parse_u32};
use crate::data::types::{Enemy, Item, ItemKind, Npc, NpcType, Quest, QuestType, Shop};

pub fn parse_items(data: &str) -> Result<Vec<Item>> {
    let mut items = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts = line.split(':').collect::<Vec<_>>();
        ensure!(parts.len() >= 6, "too few fields in item line: {}", line);

        let kind = match parts[0] {
            "W" => ItemKind::Weapon,
            "A" => ItemKind::Armor,
            "C" => ItemKind::Accessory,
            "I" => ItemKind::Consumable,
            _ => bail!("unknown item kind '{}' in: {}", parts[0], line),
        };

        let data_id = parse_u32(parts[1], "data_id", line)?;
        let id = parts[2].to_string();
        let name = parts[3].to_string();
        let param1 = parse_int(parts[4], "param1", line)?;
        let price = if kind == ItemKind::Consumable {
            parse_int(parts[5], "price", line)?
        } else {
            ensure!(
                parts.len() >= 7,
                "too few fields for equipment in: {}",
                line
            );
            parse_int(parts[6], "price", line)?
        };

        items.push(Item {
            data_id,
            id,
            name,
            kind,
            param1,
            param2: if kind == ItemKind::Consumable {
                0
            } else {
                parse_int(parts[5], "param2", line)?
            },
            price,
        });
    }

    Ok(items)
}

pub fn parse_enemies(data: &str) -> Result<Vec<Enemy>> {
    let mut enemies = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts = line.split(':').collect::<Vec<_>>();
        ensure!(parts.len() >= 8, "too few fields in enemy line: {}", line);

        let data_id = parse_u32(parts[0], "data_id", line)?;
        let hp = parse_int(parts[3], "hp", line)?;
        ensure!(hp > 0, "enemy hp must be > 0 in: {}", line);

        enemies.push(Enemy {
            data_id,
            id: parts[1].to_string(),
            name: parts[2].to_string(),
            hp,
            atk: parse_int(parts[4], "atk", line)?,
            def: parse_int(parts[5], "def", line)?,
            exp: parse_int(parts[6], "exp", line)?,
            gold: parse_int(parts[7], "gold", line)?,
        });
    }

    Ok(enemies)
}

pub fn parse_npcs(data: &str) -> Result<Vec<Npc>> {
    let mut npcs = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts = line.split(':').collect::<Vec<_>>();
        ensure!(parts.len() >= 5, "too few fields in npc line: {}", line);

        let npc_type = match parts[3] {
            "V" => NpcType::Villager,
            "S" => NpcType::ShopKeeper,
            "Q" => NpcType::QuestGiver,
            "H" => NpcType::Healer,
            _ => bail!("unknown npc type '{}' in: {}", parts[3], line),
        };

        npcs.push(Npc {
            data_id: parse_u32(parts[0], "data_id", line)?,
            id: parts[1].to_string(),
            name: parts[2].to_string(),
            map_id: String::new(),
            npc_type,
            x: 0,
            y: 0,
            dialog_id: parts[4].to_string(),
            shop_id: parts.get(5).map(|s| s.to_string()),
        });
    }

    Ok(npcs)
}

pub fn parse_quests(data: &str) -> Result<Vec<Quest>> {
    let mut quests = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts = line.split(':').collect::<Vec<_>>();
        ensure!(parts.len() >= 8, "too few fields in quest line: {}", line);

        let quest_type = match parts[2] {
            "KILL" => QuestType::Kill,
            "COLLECT" => QuestType::Collect,
            "TALK" => QuestType::Talk,
            "REACH" => QuestType::Reach,
            _ => bail!("unknown quest type '{}' in: {}", parts[2], line),
        };

        quests.push(Quest {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            quest_type,
            target_id: parts[3].to_string(),
            target_count: parse_int(parts[4], "target_count", line)?,
            reward_exp: parse_int(parts[5], "reward_exp", line)?,
            reward_gold: parse_int(parts[6], "reward_gold", line)?,
            reward_item: parts.get(8).map(|s| s.to_string()),
            description: parts[7].to_string(),
        });
    }

    Ok(quests)
}

pub fn parse_shops(data: &str) -> Result<Vec<Shop>> {
    let mut shops = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts = line.split(':').collect::<Vec<_>>();
        ensure!(parts.len() >= 3, "too few fields in shop line: {}", line);

        let items = parts[2..].iter().map(|s| s.to_string()).collect();

        shops.push(Shop {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            items,
        });
    }

    Ok(shops)
}

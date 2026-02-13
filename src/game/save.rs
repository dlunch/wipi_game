use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;
use wipi::database::{Database, OpenMode};

use super::PlayerState;
use crate::data::{Item, ItemKind, QuestProgress};

const SAVE_DB_NAME: &str = "save";

pub fn save_game(player: &PlayerState) -> Result<()> {
    let data = serialize_save(player);

    let mut db = Database::open(SAVE_DB_NAME, OpenMode::ReadWrite)
        .map_err(|e| anyhow::anyhow!("failed to open save db: {:?}", e))?;
    db.write(data.as_bytes())
        .map_err(|e| anyhow::anyhow!("failed to write save db: {:?}", e))?;
    Ok(())
}

pub fn load_game(player: &mut PlayerState) -> Result<bool> {
    let db = Database::open(SAVE_DB_NAME, OpenMode::ReadOnly)
        .map_err(|e| anyhow::anyhow!("failed to open save db: {:?}", e))?;
    let mut buf = [0u8; 4096];
    let len = db
        .read(&mut buf)
        .map_err(|e| anyhow::anyhow!("failed to read save db: {:?}", e))?;
    let data = core::str::from_utf8(&buf[..len])?;
    Ok(deserialize_save(data, player))
}

pub fn has_save_data() -> bool {
    Database::open(SAVE_DB_NAME, OpenMode::ReadOnly).is_ok()
}

fn serialize_save(player: &PlayerState) -> String {
    let mut lines = vec![
        String::from("VERSION:1"),
        format_args_to_string(&[
            "PLAYER",
            &player.name,
            &player.current_map_id,
            &player.x.to_string(),
            &player.y.to_string(),
        ]),
        format_args_to_string(&[
            "STATS",
            &player.stats.level.to_string(),
            &player.stats.exp.to_string(),
            &player.stats.max_hp.to_string(),
            &player.stats.current_hp.to_string(),
            &player.stats.max_mp.to_string(),
            &player.stats.current_mp.to_string(),
            &player.stats.base_atk.to_string(),
            &player.stats.base_def.to_string(),
            &player.stats.gold.to_string(),
        ]),
        format_args_to_string(&[
            "EQUIP",
            &player
                .equipped_weapon
                .map(|i| i.to_string())
                .unwrap_or_else(|| "-1".into()),
            &player
                .equipped_armor
                .map(|i| i.to_string())
                .unwrap_or_else(|| "-1".into()),
            &player
                .equipped_accessory
                .map(|i| i.to_string())
                .unwrap_or_else(|| "-1".into()),
        ]),
    ];

    for item in &player.inventory {
        let kind_char = match item.kind {
            ItemKind::Weapon => "W",
            ItemKind::Armor => "A",
            ItemKind::Accessory => "C",
            ItemKind::Consumable => "I",
        };
        lines.push(format_args_to_string(&[
            "ITEM",
            kind_char,
            &item.id,
            &item.name,
            &item.param1.to_string(),
            &item.param2.to_string(),
            &item.param3.to_string(),
            &item.price.to_string(),
        ]));
    }

    for quest in &player.quests {
        lines.push(format_args_to_string(&[
            "QUEST",
            &quest.quest_id,
            &quest.current_count.to_string(),
            if quest.completed { "1" } else { "0" },
            if quest.rewarded { "1" } else { "0" },
        ]));
    }

    for (map_id, x, y) in &player.opened_treasures {
        lines.push(format_args_to_string(&[
            "TREASURE",
            map_id,
            &x.to_string(),
            &y.to_string(),
        ]));
    }

    let mut result = String::new();
    for line in lines {
        result.push_str(&line);
        result.push('\n');
    }
    result
}

fn format_args_to_string(parts: &[&str]) -> String {
    let mut s = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            s.push(':');
        }
        s.push_str(part);
    }
    s
}

fn deserialize_save(data: &str, player: &mut PlayerState) -> bool {
    player.inventory.clear();
    player.quests.clear();
    player.opened_treasures.clear();

    let mut has_player = false;
    let mut has_stats = false;

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("VERSION:") {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "PLAYER" if parts.len() >= 5 => {
                player.name = parts[1].into();
                player.current_map_id = parts[2].into();
                player.x = parts[3].parse().unwrap_or(0);
                player.y = parts[4].parse().unwrap_or(0);
                has_player = true;
            }
            "STATS" if parts.len() >= 10 => {
                player.stats.level = parts[1].parse().unwrap_or(1).max(1);
                player.stats.exp = parts[2].parse().unwrap_or(0).max(0);
                player.stats.max_hp = parts[3].parse().unwrap_or(50).max(1);
                player.stats.current_hp = parts[4].parse().unwrap_or(50);
                player.stats.max_mp = parts[5].parse().unwrap_or(20).max(0);
                player.stats.current_mp = parts[6].parse().unwrap_or(20);
                player.stats.base_atk = parts[7].parse().unwrap_or(10).max(0);
                player.stats.base_def = parts[8].parse().unwrap_or(5).max(0);
                player.stats.gold = parts[9].parse().unwrap_or(0).max(0);
                player.stats.exp_to_next = player.stats.level * 100;
                player.stats.current_hp = player.stats.current_hp.clamp(0, player.stats.max_hp);
                player.stats.current_mp = player.stats.current_mp.clamp(0, player.stats.max_mp);
                has_stats = true;
            }
            "EQUIP" if parts.len() >= 4 => {
                player.equipped_weapon = parts[1]
                    .parse::<i32>()
                    .ok()
                    .filter(|&i| i >= 0)
                    .map(|i| i as usize);
                player.equipped_armor = parts[2]
                    .parse::<i32>()
                    .ok()
                    .filter(|&i| i >= 0)
                    .map(|i| i as usize);
                player.equipped_accessory = parts[3]
                    .parse::<i32>()
                    .ok()
                    .filter(|&i| i >= 0)
                    .map(|i| i as usize);
            }
            "ITEM" if parts.len() >= 8 => {
                let kind = match parts[1] {
                    "W" => ItemKind::Weapon,
                    "A" => ItemKind::Armor,
                    "C" => ItemKind::Accessory,
                    "I" => ItemKind::Consumable,
                    _ => continue,
                };
                player.inventory.push(Item {
                    id: parts[2].into(),
                    name: parts[3].into(),
                    kind,
                    param1: parts[4].parse().unwrap_or(0),
                    param2: parts[5].parse().unwrap_or(0),
                    param3: parts[6].parse().unwrap_or(0),
                    price: parts[7].parse().unwrap_or(0),
                });
            }
            "QUEST" if parts.len() >= 5 => {
                player.quests.push(QuestProgress {
                    quest_id: parts[1].into(),
                    current_count: parts[2].parse().unwrap_or(0),
                    completed: parts[3] == "1",
                    rewarded: parts[4] == "1",
                });
            }
            "TREASURE" if parts.len() >= 4 => {
                let map_id = parts[1].into();
                let x = parts[2].parse().unwrap_or(0);
                let y = parts[3].parse().unwrap_or(0);
                player.opened_treasures.push((map_id, x, y));
            }
            _ => {}
        }
    }

    if !has_player || !has_stats {
        return false;
    }

    let inv_len = player.inventory.len();
    if let Some(idx) = player.equipped_weapon
        && idx >= inv_len
    {
        player.equipped_weapon = None;
    }
    if let Some(idx) = player.equipped_armor
        && idx >= inv_len
    {
        player.equipped_armor = None;
    }
    if let Some(idx) = player.equipped_accessory
        && idx >= inv_len
    {
        player.equipped_accessory = None;
    }

    true
}

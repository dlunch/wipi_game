use alloc::string::{String, ToString};
use alloc::vec::Vec;

use wipi::database::{Database, OpenMode};

use super::Player;
use crate::data::{Item, ItemKind, QuestProgress};

const SAVE_DB_NAME: &str = "save";

pub fn save_game(player: &Player) -> bool {
    let data = serialize_save(player);

    if let Ok(mut db) = Database::open(SAVE_DB_NAME, OpenMode::ReadWrite) {
        db.write(data.as_bytes()).is_ok()
    } else {
        false
    }
}

pub fn load_game(player: &mut Player) -> bool {
    if let Ok(db) = Database::open(SAVE_DB_NAME, OpenMode::ReadOnly) {
        let mut buf = [0u8; 1024];
        if let Ok(len) = db.read(&mut buf)
            && let Ok(data) = core::str::from_utf8(&buf[..len])
        {
            return deserialize_save(data, player);
        }
    }
    false
}

pub fn has_save_data() -> bool {
    Database::open(SAVE_DB_NAME, OpenMode::ReadOnly).is_ok()
}

fn serialize_save(player: &Player) -> String {
    let mut lines = Vec::new();

    lines.push(format_args_to_string(&[
        "PLAYER",
        &player.name,
        &player.current_map_id,
        &player.x.to_string(),
        &player.y.to_string(),
    ]));

    lines.push(format_args_to_string(&[
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
    ]));

    lines.push(format_args_to_string(&[
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
    ]));

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

fn deserialize_save(data: &str, player: &mut Player) -> bool {
    player.inventory.clear();
    player.quests.clear();
    player.opened_treasures.clear();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
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
            }
            "STATS" if parts.len() >= 10 => {
                player.stats.level = parts[1].parse().unwrap_or(1);
                player.stats.exp = parts[2].parse().unwrap_or(0);
                player.stats.max_hp = parts[3].parse().unwrap_or(50);
                player.stats.current_hp = parts[4].parse().unwrap_or(50);
                player.stats.max_mp = parts[5].parse().unwrap_or(20);
                player.stats.current_mp = parts[6].parse().unwrap_or(20);
                player.stats.base_atk = parts[7].parse().unwrap_or(10);
                player.stats.base_def = parts[8].parse().unwrap_or(5);
                player.stats.gold = parts[9].parse().unwrap_or(0);
                player.stats.exp_to_next = player.stats.level * 100;
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

    true
}

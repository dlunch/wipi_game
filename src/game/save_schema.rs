use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::CharacterState;
use crate::data::{Item, ItemKind, QuestProgress};

const SAVE_VERSION: u32 = 1;

pub fn serialize(
    player: &CharacterState,
    quests: &[QuestProgress],
    opened_treasures: &[(String, usize, usize)],
) -> String {
    let mut lines = vec![
        format_args_to_string(&["VERSION", &SAVE_VERSION.to_string()]),
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

    for quest in quests {
        lines.push(format_args_to_string(&[
            "QUEST",
            &quest.quest_id,
            &quest.current_count.to_string(),
            if quest.completed { "1" } else { "0" },
            if quest.rewarded { "1" } else { "0" },
        ]));
    }

    for (map_id, x, y) in opened_treasures {
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

pub fn deserialize(
    data: &str,
    player: &mut CharacterState,
    quests: &mut Vec<QuestProgress>,
    opened_treasures: &mut Vec<(String, usize, usize)>,
) -> bool {
    let Some(normalized) = migrate_to_current_save_version(data) else {
        return false;
    };

    let mut has_player = false;
    let mut has_stats = false;

    let mut parsed_name = String::new();
    let mut parsed_map_id = String::new();
    let mut parsed_x = 0usize;
    let mut parsed_y = 0usize;

    let mut parsed_level = 1;
    let mut parsed_exp = 0;
    let mut parsed_max_hp = 50;
    let mut parsed_current_hp = 50;
    let mut parsed_max_mp = 20;
    let mut parsed_current_mp = 20;
    let mut parsed_base_atk = 10;
    let mut parsed_base_def = 5;
    let mut parsed_gold = 0;

    let mut parsed_equipped_weapon = None;
    let mut parsed_equipped_armor = None;
    let mut parsed_equipped_accessory = None;

    let mut parsed_inventory = Vec::new();
    let mut parsed_quests = Vec::new();
    let mut parsed_opened_treasures = Vec::new();

    for line in normalized.lines() {
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
                parsed_name = parts[1].into();
                parsed_map_id = parts[2].into();
                parsed_x = parts[3].parse().unwrap_or(0);
                parsed_y = parts[4].parse().unwrap_or(0);
                has_player = true;
            }
            "STATS" if parts.len() >= 10 => {
                parsed_level = parts[1].parse().unwrap_or(1).max(1);
                parsed_exp = parts[2].parse().unwrap_or(0).max(0);
                parsed_max_hp = parts[3].parse().unwrap_or(50).max(1);
                parsed_current_hp = parts[4].parse().unwrap_or(50);
                parsed_max_mp = parts[5].parse().unwrap_or(20).max(0);
                parsed_current_mp = parts[6].parse().unwrap_or(20);
                parsed_base_atk = parts[7].parse().unwrap_or(10).max(0);
                parsed_base_def = parts[8].parse().unwrap_or(5).max(0);
                parsed_gold = parts[9].parse().unwrap_or(0).max(0);
                has_stats = true;
            }
            "EQUIP" if parts.len() >= 4 => {
                parsed_equipped_weapon = parts[1]
                    .parse::<i32>()
                    .ok()
                    .filter(|&i| i >= 0)
                    .map(|i| i as usize);
                parsed_equipped_armor = parts[2]
                    .parse::<i32>()
                    .ok()
                    .filter(|&i| i >= 0)
                    .map(|i| i as usize);
                parsed_equipped_accessory = parts[3]
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
                parsed_inventory.push(Item {
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
                parsed_quests.push(QuestProgress {
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
                parsed_opened_treasures.push((map_id, x, y));
            }
            _ => {}
        }
    }

    if !has_player || !has_stats {
        return false;
    }

    let inv_len = parsed_inventory.len();
    if let Some(idx) = parsed_equipped_weapon
        && idx >= inv_len
    {
        parsed_equipped_weapon = None;
    }
    if let Some(idx) = parsed_equipped_armor
        && idx >= inv_len
    {
        parsed_equipped_armor = None;
    }
    if let Some(idx) = parsed_equipped_accessory
        && idx >= inv_len
    {
        parsed_equipped_accessory = None;
    }

    player.name = parsed_name;
    player.current_map_id = parsed_map_id;
    player.x = parsed_x;
    player.y = parsed_y;

    player.stats.level = parsed_level;
    player.stats.exp = parsed_exp;
    player.stats.max_hp = parsed_max_hp;
    player.stats.current_hp = parsed_current_hp.clamp(0, parsed_max_hp);
    player.stats.max_mp = parsed_max_mp;
    player.stats.current_mp = parsed_current_mp.clamp(0, parsed_max_mp);
    player.stats.base_atk = parsed_base_atk;
    player.stats.base_def = parsed_base_def;
    player.stats.gold = parsed_gold;
    player.stats.exp_to_next = player.stats.level * 100;

    player.inventory = parsed_inventory;
    *quests = parsed_quests;
    *opened_treasures = parsed_opened_treasures;

    player.equipped_weapon = parsed_equipped_weapon;
    player.equipped_armor = parsed_equipped_armor;
    player.equipped_accessory = parsed_equipped_accessory;

    true
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

fn migrate_to_current_save_version(data: &str) -> Option<String> {
    let (version, has_version_header) = extract_save_version(data);

    if version > SAVE_VERSION {
        return None;
    }

    if version == SAVE_VERSION {
        return Some(data.into());
    }

    if !has_version_header {
        return migrate_v0_to_v1(data);
    }

    None
}

fn extract_save_version(data: &str) -> (u32, bool) {
    let Some(first_line) = data.lines().next() else {
        return (0, false);
    };
    let line = first_line.trim();
    if !line.starts_with("VERSION:") {
        return (0, false);
    }

    let version = line
        .split(':')
        .nth(1)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    (version, true)
}

fn migrate_v0_to_v1(data: &str) -> Option<String> {
    if data.trim().is_empty() {
        return None;
    }

    let mut migrated = String::from("VERSION:1\n");
    migrated.push_str(data);
    if !data.ends_with('\n') {
        migrated.push('\n');
    }
    Some(migrated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_player() -> CharacterState {
        CharacterState::new(String::from("Hero"), "village")
    }

    #[test]
    fn deserialize_accepts_current_version() {
        let mut player = make_player();
        player.stats.level = 3;
        let quests = Vec::new();
        let opened_treasures = Vec::new();
        let save = serialize(&player, &quests, &opened_treasures);

        let mut loaded = make_player();
        let mut loaded_quests = Vec::new();
        let mut loaded_opened_treasures = Vec::new();
        assert!(deserialize(
            &save,
            &mut loaded,
            &mut loaded_quests,
            &mut loaded_opened_treasures
        ));
        assert_eq!(loaded.stats.level, 3);
    }

    #[test]
    fn deserialize_migrates_versionless_save() {
        let save = "PLAYER:Hero:village:1:2\nSTATS:2:1:50:45:20:10:10:5:7\n";
        let mut loaded = make_player();
        let mut loaded_quests = Vec::new();
        let mut loaded_opened_treasures = Vec::new();

        assert!(deserialize(
            save,
            &mut loaded,
            &mut loaded_quests,
            &mut loaded_opened_treasures
        ));
        assert_eq!(loaded.name, "Hero");
        assert_eq!(loaded.x, 1);
        assert_eq!(loaded.y, 2);
        assert_eq!(loaded.stats.level, 2);
    }

    #[test]
    fn deserialize_rejects_future_version() {
        let save = "VERSION:99\nPLAYER:Hero:village:0:0\nSTATS:1:0:50:50:20:20:10:5:0\n";
        let mut loaded = make_player();
        let mut loaded_quests = Vec::new();
        let mut loaded_opened_treasures = Vec::new();

        assert!(!deserialize(
            save,
            &mut loaded,
            &mut loaded_quests,
            &mut loaded_opened_treasures
        ));
    }
}

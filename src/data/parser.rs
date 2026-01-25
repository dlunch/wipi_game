use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::types::{
    Dialog, DialogAction, DialogCondition, DialogLine, Enemy, Item, ItemKind, Map, Npc, NpcType,
    Quest, QuestType, Shop, Tile,
};

pub fn parse_items(data: &str) -> Vec<Item> {
    let mut items = Vec::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 5 {
            continue;
        }

        let kind = match parts[0] {
            "W" => ItemKind::Weapon,
            "A" => ItemKind::Armor,
            "C" => ItemKind::Accessory,
            "I" => ItemKind::Consumable,
            _ => continue,
        };

        let id = parts[1].to_string();
        let name = parts[2].to_string();
        let param1 = parts[3].parse().unwrap_or(0);
        let param2 = parts[4].parse().unwrap_or(0);
        let (param3, price) = if kind == ItemKind::Consumable {
            (0, parts[4].parse().unwrap_or(0))
        } else {
            let p3 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
            (param2, p3)
        };

        items.push(Item {
            id,
            name,
            kind,
            param1,
            param2: if kind == ItemKind::Consumable {
                0
            } else {
                param2
            },
            param3,
            price,
        });
    }

    items
}

pub fn parse_enemies(data: &str) -> Vec<Enemy> {
    let mut enemies = Vec::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 7 {
            continue;
        }

        enemies.push(Enemy {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            hp: parts[2].parse().unwrap_or(0),
            atk: parts[3].parse().unwrap_or(0),
            def: parts[4].parse().unwrap_or(0),
            exp: parts[5].parse().unwrap_or(0),
            gold: parts[6].parse().unwrap_or(0),
        });
    }

    enemies
}

pub fn parse_maps(data: &str) -> Vec<Map> {
    let mut maps = Vec::new();
    let mut current_map: Option<MapBuilder> = None;

    for line in data.lines() {
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("@MAP:") {
            if let Some(builder) = current_map.take()
                && let Some(map) = builder.build()
            {
                maps.push(map);
            }

            let parts: Vec<&str> = rest.split(':').collect();
            let id = parts.first().map(|s| s.to_string()).unwrap_or_default();
            let name = parts
                .get(1)
                .map(|s| s.to_string())
                .unwrap_or_else(|| id.clone());

            current_map = Some(MapBuilder::new(id, name));
        } else if line == "@END" {
            if let Some(builder) = current_map.take()
                && let Some(map) = builder.build()
            {
                maps.push(map);
            }
        } else if let Some(rest) = line.strip_prefix("@ENCOUNTERS:") {
            if let Some(ref mut builder) = current_map {
                let parts: Vec<&str> = rest.split(':').collect();
                let mut i = 0;
                while i + 1 < parts.len() {
                    let enemy_id = parts[i].to_string();
                    let weight = parts[i + 1].parse().unwrap_or(1);
                    builder.encounters.push((enemy_id, weight));
                    i += 2;
                }
            }
        } else if let Some(rest) = line.strip_prefix("@NEXT:") {
            if let Some(ref mut builder) = current_map {
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() >= 3 {
                    let x = parts[0].parse().unwrap_or(0);
                    let y = parts[1].parse().unwrap_or(0);
                    let target = parts[2].to_string();
                    builder.exits.push((x, y, target));
                }
            }
        } else if let Some(rest) = line.strip_prefix("@DUNGEON:") {
            if let Some(ref mut builder) = current_map {
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() >= 3 {
                    let x = parts[0].parse().unwrap_or(0);
                    let y = parts[1].parse().unwrap_or(0);
                    let target = parts[2].to_string();
                    builder.dungeons.push((x, y, target));
                }
            }
        } else if !line.is_empty()
            && !line.starts_with('#')
            && let Some(ref mut builder) = current_map
        {
            builder.add_row(line);
        }
    }

    if let Some(builder) = current_map
        && let Some(map) = builder.build()
    {
        maps.push(map);
    }

    maps
}

pub fn parse_npcs(data: &str) -> Vec<Npc> {
    let mut npcs = Vec::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 6 {
            continue;
        }

        let npc_type = match parts[3] {
            "V" => NpcType::Villager,
            "S" => NpcType::ShopKeeper,
            "Q" => NpcType::QuestGiver,
            "H" => NpcType::Healer,
            _ => NpcType::Villager,
        };

        npcs.push(Npc {
            name: parts[1].to_string(),
            map_id: parts[2].to_string(),
            npc_type,
            x: parts[4].parse().unwrap_or(0),
            y: parts[5].parse().unwrap_or(0),
            dialog_id: parts.get(6).map(|s| s.to_string()).unwrap_or_default(),
            shop_id: parts.get(7).map(|s| s.to_string()),
        });
    }

    npcs
}

pub fn parse_dialogs(data: &str) -> Vec<Dialog> {
    let mut dialogs = Vec::new();
    let mut current: Option<DialogBuilder> = None;

    for line in data.lines() {
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("@DIALOG:") {
            if let Some(builder) = current.take() {
                dialogs.push(builder.build());
            }
            current = Some(DialogBuilder::new(rest.to_string()));
        } else if line == "@END" {
            if let Some(builder) = current.take() {
                dialogs.push(builder.build());
            }
        } else if !line.is_empty()
            && !line.starts_with('#')
            && let Some(ref mut builder) = current
        {
            builder.add_line(line);
        }
    }

    if let Some(builder) = current {
        dialogs.push(builder.build());
    }

    dialogs
}

pub fn parse_quests(data: &str) -> Vec<Quest> {
    let mut quests = Vec::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 8 {
            continue;
        }

        let quest_type = match parts[2] {
            "KILL" => QuestType::Kill,
            "COLLECT" => QuestType::Collect,
            "TALK" => QuestType::Talk,
            "REACH" => QuestType::Reach,
            _ => QuestType::Kill,
        };

        quests.push(Quest {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            quest_type,
            target_id: parts[3].to_string(),
            target_count: parts[4].parse().unwrap_or(1),
            reward_exp: parts[5].parse().unwrap_or(0),
            reward_gold: parts[6].parse().unwrap_or(0),
            reward_item: parts.get(8).map(|s| s.to_string()),
            description: parts[7].to_string(),
        });
    }

    quests
}

pub fn parse_shops(data: &str) -> Vec<Shop> {
    let mut shops = Vec::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 3 {
            continue;
        }

        let items: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();

        shops.push(Shop {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            items,
        });
    }

    shops
}

struct DialogBuilder {
    id: String,
    lines: Vec<DialogLine>,
}

impl DialogBuilder {
    fn new(id: String) -> Self {
        Self {
            id,
            lines: Vec::new(),
        }
    }

    fn add_line(&mut self, line: &str) {
        let parts: Vec<&str> = line.splitn(3, ':').collect();

        let (condition, action, text) = if parts.len() == 3 {
            (
                Self::parse_condition(parts[0]),
                Self::parse_action(parts[1]),
                parts[2].to_string(),
            )
        } else if parts.len() == 2 {
            (None, Self::parse_action(parts[0]), parts[1].to_string())
        } else {
            (None, None, line.to_string())
        };

        self.lines.push(DialogLine {
            text,
            condition,
            action,
        });
    }

    fn parse_condition(s: &str) -> Option<DialogCondition> {
        let parts: Vec<&str> = s.split('=').collect();
        if parts.len() != 2 {
            return None;
        }
        match parts[0] {
            "HAS_QUEST" => Some(DialogCondition::HasQuest(parts[1].to_string())),
            "QUEST_DONE" => Some(DialogCondition::QuestComplete(parts[1].to_string())),
            "HAS_ITEM" => Some(DialogCondition::HasItem(parts[1].to_string())),
            "HAS_GOLD" => parts[1].parse().ok().map(DialogCondition::HasGold),
            _ => None,
        }
    }

    fn parse_action(s: &str) -> Option<DialogAction> {
        let parts: Vec<&str> = s.split('=').collect();
        if parts.is_empty() {
            return None;
        }
        match parts[0] {
            "GIVE_QUEST" => parts
                .get(1)
                .map(|id| DialogAction::GiveQuest(id.to_string())),
            "COMPLETE_QUEST" => parts
                .get(1)
                .map(|id| DialogAction::CompleteQuest(id.to_string())),
            "GIVE_ITEM" => parts
                .get(1)
                .map(|id| DialogAction::GiveItem(id.to_string())),
            "TAKE_ITEM" => parts
                .get(1)
                .map(|id| DialogAction::TakeItem(id.to_string())),
            "GIVE_GOLD" => parts
                .get(1)
                .and_then(|g| g.parse().ok())
                .map(DialogAction::GiveGold),
            "TAKE_GOLD" => parts
                .get(1)
                .and_then(|g| g.parse().ok())
                .map(DialogAction::TakeGold),
            "OPEN_SHOP" => parts
                .get(1)
                .map(|id| DialogAction::OpenShop(id.to_string())),
            "HEAL" => Some(DialogAction::Heal),
            _ => None,
        }
    }

    fn build(self) -> Dialog {
        Dialog {
            id: self.id,
            lines: self.lines,
        }
    }
}

struct MapBuilder {
    id: String,
    name: String,
    rows: Vec<String>,
    encounters: Vec<(String, i32)>,
    exits: Vec<(usize, usize, String)>,
    dungeons: Vec<(usize, usize, String)>,
}

impl MapBuilder {
    fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            rows: Vec::new(),
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
        }
    }

    fn add_row(&mut self, row: &str) {
        self.rows.push(row.to_string());
    }

    fn build(self) -> Option<Map> {
        if self.rows.is_empty() {
            return None;
        }

        let height = self.rows.len();
        let width = self
            .rows
            .iter()
            .map(|r| r.chars().count())
            .max()
            .unwrap_or(0);

        let mut tiles = vec![Tile::Floor; width * height];
        let mut auto_exits = Vec::new();

        for (y, row) in self.rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let tile = Tile::from_char(c);
                tiles[y * width + x] = tile;

                if tile == Tile::Exit {
                    auto_exits.push((x, y));
                }
            }
        }

        let mut exits = self.exits;
        for (x, y) in auto_exits {
            if !exits.iter().any(|(ex, ey, _)| *ex == x && *ey == y) {
                exits.push((x, y, String::new()));
            }
        }

        Some(Map {
            id: self.id,
            name: self.name,
            width,
            height,
            tiles,
            encounters: self.encounters,
            exits,
            dungeons: self.dungeons,
        })
    }
}

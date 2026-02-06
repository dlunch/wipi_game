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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_items_weapon_and_consumable() {
        let data = "W:sword:Iron Sword:10:5:200\nI:potion:Potion:30:50\n";
        let items = parse_items(data);
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, "sword");
        assert_eq!(items[0].kind, ItemKind::Weapon);
        assert_eq!(items[0].param1, 10);
        assert_eq!(items[0].price, 200);

        assert_eq!(items[1].id, "potion");
        assert_eq!(items[1].kind, ItemKind::Consumable);
        assert_eq!(items[1].param1, 30);
        assert_eq!(items[1].price, 50);
    }

    #[test]
    fn parse_items_armor_and_accessory() {
        let data = "A:leather:Leather:5:2:150\nC:ring:Ring:3:1:300\n";
        let items = parse_items(data);
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].kind, ItemKind::Armor);
        assert_eq!(items[0].param1, 5);

        assert_eq!(items[1].kind, ItemKind::Accessory);
        assert_eq!(items[1].param1, 3);
    }

    #[test]
    fn parse_items_skips_comments_and_empty() {
        let data = "# comment\n\nW:sword:Sword:10:0:100\n";
        let items = parse_items(data);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn parse_items_skips_malformed() {
        let data = "W:too:few\nX:unknown:type:1:2:3\n";
        let items = parse_items(data);
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn parse_enemies_basic() {
        let data = "slime:Slime:20:5:2:10:5\ngoblin:Goblin:30:8:3:15:8\n";
        let enemies = parse_enemies(data);
        assert_eq!(enemies.len(), 2);
        assert_eq!(enemies[0].id, "slime");
        assert_eq!(enemies[0].hp, 20);
        assert_eq!(enemies[0].atk, 5);
        assert_eq!(enemies[1].id, "goblin");
        assert_eq!(enemies[1].gold, 8);
    }

    #[test]
    fn parse_enemies_skips_short_lines() {
        let data = "too:few:fields\n# comment\n";
        let enemies = parse_enemies(data);
        assert_eq!(enemies.len(), 0);
    }

    #[test]
    fn parse_maps_basic() {
        let data = "\
@MAP:village:Village
###
#P#
###
@ENCOUNTERS:slime:3
@NEXT:1:1:dungeon
@END
";
        let maps = parse_maps(data);
        assert_eq!(maps.len(), 1);
        let map = &maps[0];
        assert_eq!(map.id, "village");
        assert_eq!(map.name, "Village");
        assert_eq!(map.width, 3);
        assert_eq!(map.height, 3);
        assert_eq!(map.get_tile(1, 1), Tile::PlayerStart);
        assert_eq!(map.get_tile(0, 0), Tile::Wall);
        assert_eq!(map.encounters.len(), 1);
        assert_eq!(map.encounters[0].0, "slime");
        assert_eq!(map.exits.len(), 1);
        assert_eq!(map.exits[0].2, "dungeon");
    }

    #[test]
    fn parse_maps_multiple() {
        let data = "\
@MAP:a:Map A
#P
@END
@MAP:b:Map B
P#
@END
";
        let maps = parse_maps(data);
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[0].id, "a");
        assert_eq!(maps[1].id, "b");
    }

    #[test]
    fn parse_maps_auto_exit_tiles() {
        let data = "\
@MAP:test:Test
#>#
@END
";
        let maps = parse_maps(data);
        assert_eq!(maps[0].exits.len(), 1);
        assert_eq!(maps[0].exits[0].0, 1);
        assert_eq!(maps[0].exits[0].1, 0);
    }

    #[test]
    fn parse_maps_dungeon_directive() {
        let data = "\
@MAP:test:Test
#D#
@DUNGEON:1:0:cave
@END
";
        let maps = parse_maps(data);
        assert_eq!(maps[0].dungeons.len(), 1);
        assert_eq!(maps[0].dungeons[0].2, "cave");
    }

    #[test]
    fn parse_dialogs_basic() {
        let data = "\
@DIALOG:greet
Hello there!
HEAL:I'll heal you.
@END
";
        let dialogs = parse_dialogs(data);
        assert_eq!(dialogs.len(), 1);
        assert_eq!(dialogs[0].id, "greet");
        assert_eq!(dialogs[0].lines.len(), 2);
        assert_eq!(dialogs[0].lines[0].text, "Hello there!");
        assert!(dialogs[0].lines[0].action.is_none());
        assert!(matches!(
            dialogs[0].lines[1].action,
            Some(DialogAction::Heal)
        ));
    }

    #[test]
    fn parse_dialogs_with_condition_and_action() {
        let data = "\
@DIALOG:quest_npc
HAS_QUEST=slay:COMPLETE_QUEST=slay:Well done!
GIVE_QUEST=slay:Kill the slimes!
@END
";
        let dialogs = parse_dialogs(data);
        let lines = &dialogs[0].lines;
        assert_eq!(lines.len(), 2);
        assert!(matches!(
            lines[0].condition,
            Some(DialogCondition::HasQuest(_))
        ));
        assert!(matches!(
            lines[0].action,
            Some(DialogAction::CompleteQuest(_))
        ));
        assert!(matches!(lines[1].action, Some(DialogAction::GiveQuest(_))));
    }

    #[test]
    fn parse_quests_basic() {
        let data = "slay:Slay Quest:KILL:slime:5:50:20:Kill 5 slimes\n";
        let quests = parse_quests(data);
        assert_eq!(quests.len(), 1);
        assert_eq!(quests[0].id, "slay");
        assert_eq!(quests[0].quest_type, QuestType::Kill);
        assert_eq!(quests[0].target_id, "slime");
        assert_eq!(quests[0].target_count, 5);
        assert_eq!(quests[0].reward_exp, 50);
        assert_eq!(quests[0].reward_gold, 20);
        assert!(quests[0].reward_item.is_none());
    }

    #[test]
    fn parse_quests_with_reward_item() {
        let data = "q1:Quest:COLLECT:herb:3:30:10:Collect herbs:potion\n";
        let quests = parse_quests(data);
        assert_eq!(quests[0].reward_item.as_deref(), Some("potion"));
    }

    #[test]
    fn parse_shops_basic() {
        let data = "shop1:General Store:potion:sword:armor\n";
        let shops = parse_shops(data);
        assert_eq!(shops.len(), 1);
        assert_eq!(shops[0].id, "shop1");
        assert_eq!(shops[0].name, "General Store");
        assert_eq!(shops[0].items, vec!["potion", "sword", "armor"]);
    }

    #[test]
    fn parse_npcs_basic() {
        let data = "npc1:Elder:village:Q:3:5:elder_dialog\n";
        let npcs = parse_npcs(data);
        assert_eq!(npcs.len(), 1);
        assert_eq!(npcs[0].name, "Elder");
        assert_eq!(npcs[0].map_id, "village");
        assert_eq!(npcs[0].npc_type, NpcType::QuestGiver);
        assert_eq!(npcs[0].x, 3);
        assert_eq!(npcs[0].y, 5);
        assert_eq!(npcs[0].dialog_id, "elder_dialog");
    }

    #[test]
    fn parse_npcs_with_shop() {
        let data = "npc1:Merchant:town:S:2:3:shop_dialog:shop1\n";
        let npcs = parse_npcs(data);
        assert_eq!(npcs[0].npc_type, NpcType::ShopKeeper);
        assert_eq!(npcs[0].shop_id.as_deref(), Some("shop1"));
    }
}

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, bail, ensure};

use super::types::{
    Dialog, DialogAction, DialogCondition, DialogLine, Enemy, Item, ItemKind, Map, NewGameConfig,
    Npc, NpcType, Quest, QuestType, Shop, StartItem, Tile,
};

fn parse_int(s: &str, field: &str, line: &str) -> Result<i32> {
    s.parse::<i32>()
        .map_err(|_| anyhow!("invalid {} '{}' in: {}", field, s, line))
}

fn parse_usize(s: &str, field: &str, line: &str) -> Result<usize> {
    s.parse::<usize>()
        .map_err(|_| anyhow!("invalid {} '{}' in: {}", field, s, line))
}

pub fn parse_items(data: &str) -> Result<Vec<Item>> {
    let mut items = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        ensure!(parts.len() >= 5, "too few fields in item line: {}", line);

        let kind = match parts[0] {
            "W" => ItemKind::Weapon,
            "A" => ItemKind::Armor,
            "C" => ItemKind::Accessory,
            "I" => ItemKind::Consumable,
            _ => bail!("unknown item kind '{}' in: {}", parts[0], line),
        };

        let id = parts[1].to_string();
        let name = parts[2].to_string();
        let param1 = parse_int(parts[3], "param1", line)?;
        let param2 = parse_int(parts[4], "param2", line)?;
        let (param3, price) = if kind == ItemKind::Consumable {
            (0, parse_int(parts[4], "price", line)?)
        } else {
            ensure!(
                parts.len() >= 6,
                "too few fields for equipment in: {}",
                line
            );
            let p3 = parse_int(parts[5], "price", line)?;
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

    Ok(items)
}

pub fn parse_enemies(data: &str) -> Result<Vec<Enemy>> {
    let mut enemies = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        ensure!(parts.len() >= 7, "too few fields in enemy line: {}", line);

        let hp = parse_int(parts[2], "hp", line)?;
        ensure!(hp > 0, "enemy hp must be > 0 in: {}", line);

        enemies.push(Enemy {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            hp,
            atk: parse_int(parts[3], "atk", line)?,
            def: parse_int(parts[4], "def", line)?,
            exp: parse_int(parts[5], "exp", line)?,
            gold: parse_int(parts[6], "gold", line)?,
        });
    }

    Ok(enemies)
}

pub fn parse_maps(data: &str) -> Result<Vec<Map>> {
    let mut maps = Vec::new();
    let mut current_map: Option<MapBuilder> = None;

    for raw_line in data.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("@MAP:") {
            if let Some(builder) = current_map.take() {
                maps.push(builder.build()?);
            }
            ensure!(!rest.is_empty(), "missing map id in: {}", line);
            let (id_raw, name_raw) = rest.split_once(':').unwrap_or((rest, rest));
            let id = id_raw.to_string();
            let name = name_raw.to_string();

            current_map = Some(MapBuilder::new(id, name));
        } else if line == "@END" {
            if let Some(builder) = current_map.take() {
                maps.push(builder.build()?);
            }
        } else if let Some(rest) = line.strip_prefix("@ENCOUNTERS:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.split(':');
                while let Some(enemy_id) = parts.next() {
                    let Some(weight_raw) = parts.next() else {
                        break;
                    };
                    let weight = parse_int(weight_raw, "encounter weight", line)?;
                    builder.encounters.push((enemy_id.to_string(), weight));
                }
            }
        } else if let Some(rest) = line.strip_prefix("@NEXT:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.splitn(3, ':');
                let Some(x_raw) = parts.next() else {
                    bail!("too few fields in @NEXT directive: {}", line);
                };
                let Some(y_raw) = parts.next() else {
                    bail!("too few fields in @NEXT directive: {}", line);
                };
                let Some(target_raw) = parts.next() else {
                    bail!("too few fields in @NEXT directive: {}", line);
                };
                let x = parse_usize(x_raw, "exit x", line)?;
                let y = parse_usize(y_raw, "exit y", line)?;
                let target = target_raw.to_string();
                builder.exits.push((x, y, target));
            }
        } else if let Some(rest) = line.strip_prefix("@DUNGEON:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.splitn(3, ':');
                let Some(x_raw) = parts.next() else {
                    bail!("too few fields in @DUNGEON directive: {}", line);
                };
                let Some(y_raw) = parts.next() else {
                    bail!("too few fields in @DUNGEON directive: {}", line);
                };
                let Some(target_raw) = parts.next() else {
                    bail!("too few fields in @DUNGEON directive: {}", line);
                };
                let x = parse_usize(x_raw, "dungeon x", line)?;
                let y = parse_usize(y_raw, "dungeon y", line)?;
                let target = target_raw.to_string();
                builder.dungeons.push((x, y, target));
            }
        } else if line == "@PEACEFUL" {
            if let Some(ref mut builder) = current_map {
                builder.peaceful = true;
            }
        } else if let Some(rest) = line.strip_prefix("@NPC:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.splitn(3, ':');
                let Some(x_raw) = parts.next() else {
                    bail!("too few fields in @NPC directive: {}", line);
                };
                let Some(y_raw) = parts.next() else {
                    bail!("too few fields in @NPC directive: {}", line);
                };
                let Some(id_raw) = parts.next() else {
                    bail!("too few fields in @NPC directive: {}", line);
                };
                let x = parse_usize(x_raw, "npc x", line)?;
                let y = parse_usize(y_raw, "npc y", line)?;
                let npc_id = id_raw.to_string();
                builder.npcs.push((x, y, npc_id));
            }
        } else if !line.is_empty()
            && let Some(ref mut builder) = current_map
        {
            builder.add_row(line);
        }
    }

    if let Some(builder) = current_map {
        maps.push(builder.build()?);
    }

    Ok(maps)
}

pub fn parse_npcs(data: &str) -> Result<Vec<Npc>> {
    let mut npcs = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        ensure!(parts.len() >= 4, "too few fields in npc line: {}", line);

        let npc_type = match parts[2] {
            "V" => NpcType::Villager,
            "S" => NpcType::ShopKeeper,
            "Q" => NpcType::QuestGiver,
            "H" => NpcType::Healer,
            _ => bail!("unknown npc type '{}' in: {}", parts[2], line),
        };

        npcs.push(Npc {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            map_id: String::new(),
            npc_type,
            x: 0,
            y: 0,
            dialog_id: parts[3].to_string(),
            shop_id: parts.get(4).map(|s| s.to_string()),
        });
    }

    Ok(npcs)
}

pub fn parse_dialogs(data: &str) -> Result<Vec<Dialog>> {
    let mut dialogs = Vec::new();
    let mut current: Option<DialogBuilder> = None;

    for raw_line in data.lines() {
        let line = raw_line.trim();

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

    Ok(dialogs)
}

pub fn parse_quests(data: &str) -> Result<Vec<Quest>> {
    let mut quests = Vec::new();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
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

        let parts: Vec<&str> = line.split(':').collect();
        ensure!(parts.len() >= 3, "too few fields in shop line: {}", line);

        let items: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();

        shops.push(Shop {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            items,
        });
    }

    Ok(shops)
}

pub fn parse_newgame(data: &str) -> Result<NewGameConfig> {
    let mut config = NewGameConfig::default();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ':').collect();
        ensure!(!parts.is_empty(), "empty directive in newgame config");

        match parts[0] {
            "player_name" => {
                ensure!(parts.len() >= 2, "missing value for player_name");
                config.player_name = parts[1].to_string();
            }
            "start_map" => {
                ensure!(parts.len() >= 2, "missing value for start_map");
                config.start_map = parts[1].to_string();
            }
            "fallback_map" => {
                ensure!(parts.len() >= 2, "missing value for fallback_map");
                config.fallback_map = parts[1].to_string();
            }
            "intro_dialog" => {
                ensure!(
                    parts.len() >= 3,
                    "intro_dialog requires dialog_id and npc_name"
                );
                config.intro_dialog = Some((parts[1].to_string(), parts[2].to_string()));
            }
            "equip_weapon" => {
                ensure!(parts.len() >= 2, "missing value for equip_weapon");
                config.equip_weapon = Some(parts[1].to_string());
            }
            "equip_armor" => {
                ensure!(parts.len() >= 2, "missing value for equip_armor");
                config.equip_armor = Some(parts[1].to_string());
            }
            "treasure_item" => {
                ensure!(parts.len() >= 2, "missing value for treasure_item");
                config.treasure_item = Some(parts[1].to_string());
            }
            "item" => {
                ensure!(parts.len() >= 3, "item requires item_id and count");
                let count = parse_int(parts[2], "item count", line)?;
                config.items.push(StartItem {
                    item_id: parts[1].to_string(),
                    count,
                });
            }
            _ => bail!(
                "unknown directive '{}' in newgame config: {}",
                parts[0],
                line
            ),
        }
    }

    Ok(config)
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
    npcs: Vec<(usize, usize, String)>,
    peaceful: bool,
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
            npcs: Vec::new(),
            peaceful: false,
        }
    }

    fn add_row(&mut self, row: &str) {
        self.rows.push(row.to_string());
    }

    fn build(self) -> Result<Map> {
        ensure!(!self.rows.is_empty(), "map '{}' has no tile rows", self.id);

        let height = self.rows.len();
        let width = self.rows[0].len();

        let mut tiles = vec![Tile::Floor; width * height];
        let mut auto_exits = Vec::new();

        for (y, row) in self.rows.iter().enumerate() {
            let row_width = row.len();
            ensure!(
                row_width <= width,
                "map '{}' row {} is longer than first row ({} > {})",
                self.id,
                y,
                row_width,
                width
            );
            for (x, b) in row.bytes().enumerate() {
                let tile = Tile::from_char(b as char);
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

        Ok(Map {
            id: self.id,
            name: self.name,
            width,
            height,
            tiles,
            encounters: self.encounters,
            exits,
            dungeons: self.dungeons,
            npcs: self.npcs,
            peaceful: self.peaceful,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_items_weapon_and_consumable() -> Result<()> {
        let data = "W:sword:Iron Sword:10:5:200\nI:potion:Potion:30:50\n";
        let items = parse_items(data)?;
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, "sword");
        assert_eq!(items[0].kind, ItemKind::Weapon);
        assert_eq!(items[0].param1, 10);
        assert_eq!(items[0].price, 200);

        assert_eq!(items[1].id, "potion");
        assert_eq!(items[1].kind, ItemKind::Consumable);
        assert_eq!(items[1].param1, 30);
        assert_eq!(items[1].price, 50);
        Ok(())
    }

    #[test]
    fn parse_items_armor_and_accessory() -> Result<()> {
        let data = "A:leather:Leather:5:2:150\nC:ring:Ring:3:1:300\n";
        let items = parse_items(data)?;
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].kind, ItemKind::Armor);
        assert_eq!(items[0].param1, 5);

        assert_eq!(items[1].kind, ItemKind::Accessory);
        assert_eq!(items[1].param1, 3);
        Ok(())
    }

    #[test]
    fn parse_items_skips_comments_and_empty() -> Result<()> {
        let data = "# comment\n\nW:sword:Sword:10:0:100\n";
        let items = parse_items(data)?;
        assert_eq!(items.len(), 1);
        Ok(())
    }

    #[test]
    fn parse_items_rejects_too_few_fields() {
        let data = "W:too:few\n";
        assert!(parse_items(data).is_err());
    }

    #[test]
    fn parse_items_rejects_unknown_kind() {
        let data = "X:unknown:type:1:2:3\n";
        assert!(parse_items(data).is_err());
    }

    #[test]
    fn parse_items_rejects_bad_number() {
        let data = "W:sword:Sword:abc:5:200\n";
        assert!(parse_items(data).is_err());
    }

    #[test]
    fn parse_enemies_basic() -> Result<()> {
        let data = "slime:Slime:20:5:2:10:5\ngoblin:Goblin:30:8:3:15:8\n";
        let enemies = parse_enemies(data)?;
        assert_eq!(enemies.len(), 2);
        assert_eq!(enemies[0].id, "slime");
        assert_eq!(enemies[0].hp, 20);
        assert_eq!(enemies[0].atk, 5);
        assert_eq!(enemies[1].id, "goblin");
        assert_eq!(enemies[1].gold, 8);
        Ok(())
    }

    #[test]
    fn parse_enemies_rejects_short_lines() {
        let data = "too:few:fields\n";
        assert!(parse_enemies(data).is_err());
    }

    #[test]
    fn parse_enemies_rejects_zero_hp() {
        let data = "slime:Slime:0:5:2:10:5\n";
        assert!(parse_enemies(data).is_err());
    }

    #[test]
    fn parse_maps_basic() -> Result<()> {
        let data = "\
@MAP:village:Village
###
#P#
###
@ENCOUNTERS:slime:3
@NEXT:1:1:dungeon
@END
";
        let maps = parse_maps(data)?;
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
        Ok(())
    }

    #[test]
    fn parse_maps_multiple() -> Result<()> {
        let data = "\
@MAP:a:Map A
#P
@END
@MAP:b:Map B
P#
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[0].id, "a");
        assert_eq!(maps[1].id, "b");
        Ok(())
    }

    #[test]
    fn parse_maps_auto_exit_tiles() -> Result<()> {
        let data = "\
@MAP:test:Test
#>#
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps[0].exits.len(), 1);
        assert_eq!(maps[0].exits[0].0, 1);
        assert_eq!(maps[0].exits[0].1, 0);
        Ok(())
    }

    #[test]
    fn parse_maps_dungeon_directive() -> Result<()> {
        let data = "\
@MAP:test:Test
#D#
@DUNGEON:1:0:cave
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps[0].dungeons.len(), 1);
        assert_eq!(maps[0].dungeons[0].2, "cave");
        Ok(())
    }

    #[test]
    fn parse_maps_npc_directive() -> Result<()> {
        let data = "\
@MAP:test:Test
#P#
@NPC:1:0:elder
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps[0].npcs.len(), 1);
        assert_eq!(maps[0].npcs[0].0, 1);
        assert_eq!(maps[0].npcs[0].1, 0);
        assert_eq!(maps[0].npcs[0].2, "elder");
        Ok(())
    }

    #[test]
    fn parse_maps_rejects_row_longer_than_first_row() {
        let data = r#"
@MAP:test:Test
##
###
@END
"#;

        assert!(parse_maps(data).is_err());
    }

    #[test]
    fn parse_dialogs_basic() -> Result<()> {
        let data = "\
@DIALOG:greet
Hello there!
HEAL:I'll heal you.
@END
";
        let dialogs = parse_dialogs(data)?;
        assert_eq!(dialogs.len(), 1);
        assert_eq!(dialogs[0].id, "greet");
        assert_eq!(dialogs[0].lines.len(), 2);
        assert_eq!(dialogs[0].lines[0].text, "Hello there!");
        assert!(dialogs[0].lines[0].action.is_none());
        assert!(matches!(
            dialogs[0].lines[1].action,
            Some(DialogAction::Heal)
        ));
        Ok(())
    }

    #[test]
    fn parse_dialogs_with_condition_and_action() -> Result<()> {
        let data = "\
@DIALOG:quest_npc
HAS_QUEST=slay:COMPLETE_QUEST=slay:Well done!
GIVE_QUEST=slay:Kill the slimes!
@END
";
        let dialogs = parse_dialogs(data)?;
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
        Ok(())
    }

    #[test]
    fn parse_quests_basic() -> Result<()> {
        let data = "slay:Slay Quest:KILL:slime:5:50:20:Kill 5 slimes\n";
        let quests = parse_quests(data)?;
        assert_eq!(quests.len(), 1);
        assert_eq!(quests[0].id, "slay");
        assert_eq!(quests[0].quest_type, QuestType::Kill);
        assert_eq!(quests[0].target_id, "slime");
        assert_eq!(quests[0].target_count, 5);
        assert_eq!(quests[0].reward_exp, 50);
        assert_eq!(quests[0].reward_gold, 20);
        assert!(quests[0].reward_item.is_none());
        Ok(())
    }

    #[test]
    fn parse_quests_with_reward_item() -> Result<()> {
        let data = "q1:Quest:COLLECT:herb:3:30:10:Collect herbs:potion\n";
        let quests = parse_quests(data)?;
        assert_eq!(quests[0].reward_item.as_deref(), Some("potion"));
        Ok(())
    }

    #[test]
    fn parse_quests_rejects_unknown_type() {
        let data = "q1:Quest:INVALID:x:1:10:5:desc\n";
        assert!(parse_quests(data).is_err());
    }

    #[test]
    fn parse_shops_basic() -> Result<()> {
        let data = "shop1:General Store:potion:sword:armor\n";
        let shops = parse_shops(data)?;
        assert_eq!(shops.len(), 1);
        assert_eq!(shops[0].id, "shop1");
        assert_eq!(shops[0].name, "General Store");
        assert_eq!(shops[0].items, vec!["potion", "sword", "armor"]);
        Ok(())
    }

    #[test]
    fn parse_npcs_basic() -> Result<()> {
        let data = "npc1:Elder:Q:elder_dialog\n";
        let npcs = parse_npcs(data)?;
        assert_eq!(npcs.len(), 1);
        assert_eq!(npcs[0].id, "npc1");
        assert_eq!(npcs[0].name, "Elder");
        assert_eq!(npcs[0].npc_type, NpcType::QuestGiver);
        assert_eq!(npcs[0].dialog_id, "elder_dialog");
        Ok(())
    }

    #[test]
    fn parse_npcs_with_shop() -> Result<()> {
        let data = "npc1:Merchant:S:shop_dialog:shop1\n";
        let npcs = parse_npcs(data)?;
        assert_eq!(npcs[0].npc_type, NpcType::ShopKeeper);
        assert_eq!(npcs[0].shop_id.as_deref(), Some("shop1"));
        Ok(())
    }

    #[test]
    fn parse_npcs_rejects_unknown_type() {
        let data = "npc1:Test:Z:dialog\n";
        assert!(parse_npcs(data).is_err());
    }

    #[test]
    fn parse_newgame_full_config() -> Result<()> {
        let data = "player_name:Hero\nstart_map:village\nfallback_map:village\nintro_dialog:dialog_guide:Guide NPC\nequip_weapon:wooden_sword\nequip_armor:cloth\ntreasure_item:elixir\nitem:potion:2\nitem:elixir:1\n";
        let config = parse_newgame(data)?;
        assert_eq!(config.player_name, "Hero");
        assert_eq!(config.start_map, "village");
        assert_eq!(config.fallback_map, "village");
        assert_eq!(
            config.intro_dialog,
            Some(("dialog_guide".into(), "Guide NPC".into()))
        );
        assert_eq!(config.equip_weapon.as_deref(), Some("wooden_sword"));
        assert_eq!(config.equip_armor.as_deref(), Some("cloth"));
        assert_eq!(config.treasure_item.as_deref(), Some("elixir"));
        assert_eq!(config.items.len(), 2);
        assert_eq!(config.items[0].item_id, "potion");
        assert_eq!(config.items[0].count, 2);
        assert_eq!(config.items[1].item_id, "elixir");
        assert_eq!(config.items[1].count, 1);
        Ok(())
    }

    #[test]
    fn parse_newgame_defaults() -> Result<()> {
        let data = "";
        let config = parse_newgame(data)?;
        assert_eq!(config.player_name, "Hero");
        assert_eq!(config.start_map, "village");
        assert!(config.intro_dialog.is_none());
        assert!(config.equip_weapon.is_none());
        assert_eq!(config.treasure_item.as_deref(), Some("potion"));
        assert!(config.items.is_empty());
        Ok(())
    }

    #[test]
    fn parse_newgame_skips_comments_and_empty() -> Result<()> {
        let data = "# comment\n\nplayer_name:Test\n";
        let config = parse_newgame(data)?;
        assert_eq!(config.player_name, "Test");
        Ok(())
    }

    #[test]
    fn parse_newgame_rejects_unknown_directive() {
        let data = "unknown_key:value\n";
        assert!(parse_newgame(data).is_err());
    }

    #[test]
    fn parse_newgame_rejects_bad_item_count() {
        let data = "item:potion:abc\n";
        assert!(parse_newgame(data).is_err());
    }
}
